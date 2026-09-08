use core::{error::Error, fmt};

use runenui_core::{
    Brush, Color, LogicalRect, LogicalTransform, PaintPrimitive, ResourceKind, SceneOpacity,
    SceneShape, StrokeJoin, StrokeStyle,
};
use runenui_runtime::{PaintPublication, PaintSceneItem, SceneCapabilities};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedSceneSemantic {
    Stroke,
    NonRectShape,
    NonSolidBrush,
    StrokeStyle,
    Image,
    ShapedTextRun,
    UnknownPrimitive,
    NonEmptyClips,
    EllipseClip,
    PathClip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneValidationError {
    UnsupportedResourceKind {
        resource_kind: ResourceKind,
    },
    UnsupportedItem {
        item_index: usize,
        semantic: UnsupportedSceneSemantic,
    },
}

impl fmt::Display for SceneValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedResourceKind { resource_kind } => write!(
                formatter,
                "renderer does not support scene resource kind {resource_kind:?}"
            ),
            Self::UnsupportedItem {
                item_index,
                semantic,
            } => write!(
                formatter,
                "renderer rejects unsupported scene item {item_index}: {semantic:?}"
            ),
        }
    }
}

impl Error for SceneValidationError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportedFillRect {
    pub(crate) rect: LogicalRect,
    pub(crate) color: Color,
    pub(crate) opacity: SceneOpacity,
    pub(crate) local_to_surface: LogicalTransform,
}

/// One renderer-admitted literal rectangle item represented by a color-bearing
/// rectangle plus an optional inner rectangle that must be excluded.
///
/// The public scene vocabulary is generic M9 fill/stroke. This temporary renderer
/// checkpoint admits only rectangular solid brushes and, for strokes, the exact
/// sharp-miter subset represented by the existing literal rectangle mask path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SupportedLiteralRect {
    pub(crate) fill: SupportedFillRect,
    pub(crate) stroke_inset: Option<LogicalRect>,
}

pub(crate) fn publication_resource_error(
    publication: &PaintPublication,
) -> Option<SceneValidationError> {
    let requirements = publication.scene().requirements();
    SceneCapabilities::default()
        .check_requirements(&requirements)
        .err()
        .map(|error| SceneValidationError::UnsupportedResourceKind {
            resource_kind: error.resource_kind(),
        })
}

/// Validates one currently realized generic fill/stroke item without applying
/// the temporary base-renderer clip gate.
///
/// Generic public semantics are not narrowed here: unsupported shapes, brushes,
/// and stroke styles fail closed until their M9A realization lands.
pub(crate) const fn validate_literal_rect_item(
    item_index: usize,
    item: &PaintSceneItem,
) -> Result<Option<SupportedLiteralRect>, SceneValidationError> {
    match item.primitive() {
        PaintPrimitive::Fill { shape, brush } => {
            let SceneShape::Rect(rect) = shape else {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonRectShape,
                ));
            };
            let Brush::Solid(color) = brush else {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonSolidBrush,
                ));
            };
            Ok(Some(SupportedLiteralRect {
                fill: supported_fill_rect(item, *rect, *color),
                stroke_inset: None,
            }))
        }
        PaintPrimitive::Stroke {
            shape,
            brush,
            style,
        } => {
            let SceneShape::Rect(rect) = shape else {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonRectShape,
                ));
            };
            let Brush::Solid(color) = brush else {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonSolidBrush,
                ));
            };
            if !supports_literal_rect_stroke(*style) {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::StrokeStyle,
                ));
            }
            Ok(supported_stroke_rect(
                item,
                *rect,
                *color,
                style.width().get(),
            ))
        }
        PaintPrimitive::Image(_) => Err(unsupported(item_index, UnsupportedSceneSemantic::Image)),
        PaintPrimitive::ShapedTextRun(_) => Err(unsupported(
            item_index,
            UnsupportedSceneSemantic::ShapedTextRun,
        )),
        _ => Err(unsupported(
            item_index,
            UnsupportedSceneSemantic::UnknownPrimitive,
        )),
    }
}

const fn supports_literal_rect_stroke(style: StrokeStyle) -> bool {
    matches!(style.join(), StrokeJoin::Miter) && style.miter_limit() >= 1.414_213_5
}

const fn supported_fill_rect(
    item: &PaintSceneItem,
    rect: LogicalRect,
    color: Color,
) -> SupportedFillRect {
    SupportedFillRect {
        rect,
        color,
        opacity: item.opacity(),
        local_to_surface: item.local_to_surface(),
    }
}

const fn supported_stroke_rect(
    item: &PaintSceneItem,
    rect: LogicalRect,
    color: Color,
    width: f32,
) -> Option<SupportedLiteralRect> {
    if width == 0.0 || rect.width() == 0.0 || rect.height() == 0.0 {
        return None;
    }

    let half = width / 2.0;
    let Ok(expanded) = LogicalRect::try_new(
        rect.x() - half,
        rect.y() - half,
        rect.width() + width,
        rect.height() + width,
    ) else {
        return None;
    };

    let stroke_inset = if rect.width() <= width || rect.height() <= width {
        None
    } else {
        match LogicalRect::try_new(
            rect.x() + half,
            rect.y() + half,
            rect.width() - width,
            rect.height() - width,
        ) {
            Ok(inset) => Some(inset),
            Err(_) => return None,
        }
    };

    Some(SupportedLiteralRect {
        fill: supported_fill_rect(item, expanded, color),
        stroke_inset,
    })
}

pub(crate) const fn validate_fill_rect_item(
    item_index: usize,
    item: &PaintSceneItem,
) -> Result<SupportedFillRect, SceneValidationError> {
    if matches!(item.primitive(), PaintPrimitive::Stroke { .. }) {
        return Err(unsupported(item_index, UnsupportedSceneSemantic::Stroke));
    }
    match validate_literal_rect_item(item_index, item) {
        Ok(Some(literal)) => Ok(literal.fill),
        Ok(None) => Err(unsupported(item_index, UnsupportedSceneSemantic::Stroke)),
        Err(error) => Err(error),
    }
}

/// Validates the complete publication before any target or GPU work begins.
///
/// `SceneCapabilities` remains the canonical resource-kind check. The following
/// item walk is deliberately renderer-owned and describes only this temporary
/// implementation checkpoint, not runtime scene capability authority.
pub fn validate_scene_subset(
    publication: &PaintPublication,
) -> Result<Vec<SupportedFillRect>, SceneValidationError> {
    let unsupported_resource_kind = publication_resource_error(publication);

    let fill_rects = publication
        .scene()
        .items()
        .iter()
        .enumerate()
        .map(|(item_index, item)| {
            let fill = validate_fill_rect_item(item_index, item)?;
            if !item.clips().is_empty() {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonEmptyClips,
                ));
            }
            Ok(fill)
        })
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(error) = unsupported_resource_kind {
        return Err(error);
    }
    Ok(fill_rects)
}

const fn unsupported(
    item_index: usize,
    semantic: UnsupportedSceneSemantic,
) -> SceneValidationError {
    SceneValidationError::UnsupportedItem {
        item_index,
        semantic,
    }
}
