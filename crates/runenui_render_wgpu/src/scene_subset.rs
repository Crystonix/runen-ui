use core::{error::Error, fmt};

use runenui_core::{Color, LogicalRect, LogicalTransform, ResourceKind, SceneOpacity};
use runenui_runtime::PaintSceneItem;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnsupportedSceneSemantic {
    Image,
    UnknownPrimitive,
    EllipseClip,
    PathClip,
    CompositionGroup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SceneValidationError {
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
pub(crate) struct SupportedFillRect {
    pub(crate) rect: LogicalRect,
    pub(crate) color: Color,
    pub(crate) opacity: SceneOpacity,
    pub(crate) local_to_surface: LogicalTransform,
}

/// Fails closed while the current wgpu implementation cannot realize atomic
/// composition groups. This is renderer capability admission only; the runtime
/// scene remains the complete neutral authority.
pub(crate) const fn validate_item_composition(
    item_index: usize,
    item: &PaintSceneItem,
) -> Result<(), SceneValidationError> {
    if item.group().is_some() {
        Err(SceneValidationError::UnsupportedItem {
            item_index,
            semantic: UnsupportedSceneSemantic::CompositionGroup,
        })
    } else {
        Ok(())
    }
}
