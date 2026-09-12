//! Renderer-private symbolic neutral effect-support reconstruction.
//!
//! ADR 0015 separates ordinary-shadow support from realized color alpha. This
//! module therefore retains only renderer-neutral publication geometry and exact
//! set operations. It deliberately carries no brush/foreground alpha, image
//! payload alpha, item/group opacity, antialiasing, MSDF samples, raster scale,
//! target extent, or cache/device state. Visual rasterization remains a later
//! disposable realization of this symbolic support; nested groups must propagate
//! this expression rather than rendered pixels.

use std::sync::Arc;

use runenui_core::{
    LogicalPoint, LogicalRect, LogicalTransform, PaintPrimitive, ResourceRef, SceneShape,
    StrokeStyle,
};
use runenui_runtime::{PaintSceneItem, SceneClip};

use crate::scene_subset::UnsupportedSceneSemantic;

/// Renderer-private scalar facts frozen from one runtime-published ordinary shadow.
///
/// The renderer does not retain the authored/runtime `DropShadow` vocabulary as a
/// second behavior authority. Only the geometry needed by ADR 0015's neutral
/// support operation crosses this private realization seam. Shadow color is
/// intentionally absent because it cannot change neutral support.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NeutralShadowFacts {
    spread: f64,
    offset_x: f64,
    offset_y: f64,
    blur_square_half_extent: f64,
}

impl NeutralShadowFacts {
    pub(super) fn new(offset_x: f32, offset_y: f32, sigma: f32, spread: f32) -> Self {
        Self {
            spread: f64::from(spread),
            offset_x: f64::from(offset_x),
            offset_y: f64::from(offset_y),
            blur_square_half_extent: f64::from(sigma) * 3.0,
        }
    }
}

/// One direct primitive's alpha-independent neutral support authority.
#[allow(
    dead_code,
    reason = "the complete primitive facts are prepared now and consumed by the immediately following ordinary-shadow visual realization checkpoint"
)]
#[derive(Clone, Debug, PartialEq)]
pub(super) enum NeutralPrimitiveSupport {
    Fill {
        shape: SceneShape,
        local_to_surface: LogicalTransform,
    },
    Stroke {
        shape: SceneShape,
        style: StrokeStyle,
        local_to_surface: LogicalTransform,
    },
    Image {
        destinations: Arc<[LogicalRect]>,
        local_to_surface: LogicalTransform,
    },
    ShapedText {
        resource: ResourceRef,
        origin: LogicalPoint,
        local_to_surface: LogicalTransform,
    },
}

/// Exact symbolic neutral-support set used by ordinary-shadow preparation.
///
/// `Shadow` denotes ADR 0015's support operation only: signed Euclidean spread,
/// then offset, then Minkowski sum with the closed axis-aligned `3 * sigma`
/// square. Shadow color is intentionally absent because alpha cannot shrink
/// neutral support. `Clip` is outside child-plus-shadow union, matching group
/// effect ordering.
#[allow(
    dead_code,
    reason = "symbolic support is reconstructed and structurally validated in this checkpoint before the next checkpoint consumes every field for disposable visual realization"
)]
#[derive(Clone, Debug, PartialEq)]
pub(super) enum NeutralSupport {
    Empty,
    Primitive(NeutralPrimitiveSupport),
    Union(Arc<[Arc<Self>]>),
    Shadow {
        source: Arc<Self>,
        spread: f64,
        offset_x: f64,
        offset_y: f64,
        blur_square_half_extent: f64,
    },
    Clip {
        source: Arc<Self>,
        clips: Arc<[SceneClip]>,
    },
}

impl NeutralSupport {
    /// Reconstructs one published item's exact neutral support without observing
    /// any realized source alpha or item opacity.
    pub(super) fn from_item(item: &PaintSceneItem) -> Result<Arc<Self>, UnsupportedSceneSemantic> {
        let local_to_surface = item.local_to_surface();
        let primitive = match item.primitive() {
            PaintPrimitive::Fill { shape, .. } => NeutralPrimitiveSupport::Fill {
                shape: shape.clone(),
                local_to_surface,
            },
            PaintPrimitive::Stroke { shape, style, .. } => NeutralPrimitiveSupport::Stroke {
                shape: shape.clone(),
                style: *style,
                local_to_surface,
            },
            PaintPrimitive::Image(image) => {
                let patch_count = image
                    .resolved_patch_count()
                    .ok_or(UnsupportedSceneSemantic::Image)?;
                let destinations = (0..patch_count)
                    .map(|patch_index| {
                        image
                            .resolved_patch(patch_index)
                            .map(|(_, destination)| destination)
                            .unwrap_or_else(|| {
                                unreachable!("runtime-resolved image patch count is exact")
                            })
                    })
                    .collect::<Vec<_>>();
                if destinations.is_empty() {
                    return Ok(Arc::new(Self::Empty));
                }
                NeutralPrimitiveSupport::Image {
                    destinations: destinations.into(),
                    local_to_surface,
                }
            }
            PaintPrimitive::ShapedTextRun(run) => NeutralPrimitiveSupport::ShapedText {
                resource: run.resource_ref().clone(),
                origin: run.origin(),
                local_to_surface,
            },
            _ => return Err(UnsupportedSceneSemantic::UnknownPrimitive),
        };
        Ok(Self::clipped(
            Arc::new(Self::Primitive(primitive)),
            item.clips(),
        ))
    }

    /// Unions direct child support without introducing painter-order authority.
    /// Runtime ordering remains independently frozen by `PaintSceneEntry`.
    pub(super) fn union(supports: impl IntoIterator<Item = Arc<Self>>) -> Arc<Self> {
        let members = supports
            .into_iter()
            .filter(|support| !matches!(support.as_ref(), Self::Empty))
            .collect::<Vec<_>>();
        match members.as_slice() {
            [] => Arc::new(Self::Empty),
            [single] => Arc::clone(single),
            _ => Arc::new(Self::Union(members.into())),
        }
    }

    /// Builds one group's ADR 0015 output support. Every sibling shadow receives
    /// the same exact pre-shadow child-support allocation; no shadow chains from a
    /// previous sibling's result.
    pub(super) fn group(
        children: impl IntoIterator<Item = Arc<Self>>,
        shadows: impl IntoIterator<Item = NeutralShadowFacts>,
        clips: &[SceneClip],
    ) -> Arc<Self> {
        let child = Self::union(children);
        if matches!(child.as_ref(), Self::Empty) {
            return child;
        }

        let shadows = shadows.into_iter().collect::<Vec<_>>();
        let mut output_members = Vec::with_capacity(shadows.len().saturating_add(1));
        output_members.push(Arc::clone(&child));
        output_members.extend(
            shadows
                .into_iter()
                .map(|shadow| Self::shadow(Arc::clone(&child), shadow)),
        );
        Self::clipped(Self::union(output_members), clips)
    }

    fn shadow(source: Arc<Self>, shadow: NeutralShadowFacts) -> Arc<Self> {
        if matches!(source.as_ref(), Self::Empty) {
            return source;
        }
        Arc::new(Self::Shadow {
            source,
            spread: shadow.spread,
            offset_x: shadow.offset_x,
            offset_y: shadow.offset_y,
            blur_square_half_extent: shadow.blur_square_half_extent,
        })
    }

    fn clipped(source: Arc<Self>, clips: &[SceneClip]) -> Arc<Self> {
        if clips.is_empty() || matches!(source.as_ref(), Self::Empty) {
            source
        } else {
            Arc::new(Self::Clip {
                source,
                clips: clips.to_vec().into(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use runenui_core::{Color, DropShadow, LogicalLength, LogicalRect, LogicalTransform};

    use super::{NeutralPrimitiveSupport, NeutralShadowFacts, NeutralSupport};

    fn rect() -> LogicalRect {
        LogicalRect::try_new(0.0, 0.0, 8.0, 6.0)
            .unwrap_or_else(|_| unreachable!("controlled rectangle is valid"))
    }

    fn source() -> Arc<NeutralSupport> {
        Arc::new(NeutralSupport::Primitive(NeutralPrimitiveSupport::Image {
            destinations: Arc::<[LogicalRect]>::from(vec![rect()]),
            local_to_surface: LogicalTransform::IDENTITY,
        }))
    }

    fn shadow(color: Color) -> DropShadow {
        DropShadow::new(
            2.0,
            -3.0,
            LogicalLength::new(4.0).unwrap_or_else(|_| unreachable!("controlled sigma is valid")),
            -1.5,
            color,
        )
        .unwrap_or_else(|_| unreachable!("controlled shadow is valid"))
    }

    fn facts(shadow: DropShadow) -> NeutralShadowFacts {
        NeutralShadowFacts::new(
            shadow.offset_x(),
            shadow.offset_y(),
            shadow.sigma().get(),
            shadow.spread(),
        )
    }

    #[test]
    fn shadow_color_alpha_cannot_change_neutral_support() {
        let transparent =
            NeutralSupport::shadow(source(), facts(shadow(Color::rgba(0x10, 0x20, 0x30, 0x00))));
        let opaque =
            NeutralSupport::shadow(source(), facts(shadow(Color::rgba(0xF0, 0xE0, 0xD0, 0xFF))));
        assert_eq!(transparent, opaque);
    }

    #[test]
    fn sibling_shadows_share_one_pre_shadow_source() {
        let child = source();
        let shadows = [
            facts(shadow(Color::rgba(0x00, 0x00, 0x00, 0x40))),
            facts(
                DropShadow::new(
                    -1.0,
                    5.0,
                    LogicalLength::new(2.0)
                        .unwrap_or_else(|_| unreachable!("controlled sigma is valid")),
                    3.0,
                    Color::rgba(0xFF, 0x00, 0x00, 0x80),
                )
                .unwrap_or_else(|_| unreachable!("controlled shadow is valid")),
            ),
        ];
        let support = NeutralSupport::group([Arc::clone(&child)], shadows, &[]);
        let NeutralSupport::Union(members) = support.as_ref() else {
            panic!("child plus two shadows must remain a support union");
        };
        assert_eq!(members.len(), 3);
        assert!(Arc::ptr_eq(&members[0], &child));
        for member in &members[1..] {
            let NeutralSupport::Shadow { source, .. } = member.as_ref() else {
                panic!("every authored sibling shadow must retain one shadow operation");
            };
            assert!(Arc::ptr_eq(source, &child));
        }
    }

    #[test]
    fn neutral_shadow_support_freezes_spread_offset_and_three_sigma_envelope() {
        let support =
            NeutralSupport::shadow(source(), facts(shadow(Color::rgba(0x00, 0x00, 0x00, 0x80))));
        let NeutralSupport::Shadow {
            spread,
            offset_x,
            offset_y,
            blur_square_half_extent,
            ..
        } = support.as_ref()
        else {
            panic!("controlled shadow must produce one symbolic shadow operation");
        };
        assert_eq!(*spread, -1.5);
        assert_eq!(*offset_x, 2.0);
        assert_eq!(*offset_y, -3.0);
        assert_eq!(*blur_square_half_extent, 12.0);
    }
}
