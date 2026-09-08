//! Renderer-neutral owner-local paint contribution vocabulary.

use crate::{
    Brush, Color, ComputedStyle, ContributionClip, ImageIntrinsicSize, ImagePaintDescriptor,
    LogicalPoint, LogicalRect, LogicalSize, LogicalTransform, ResourceKind, ResourceKindMismatch,
    ResourceRef, SceneLayer, SceneOpacity, SceneShape, StrokeStyle,
};

/// Read-only facts supplied while one mounted widget contributes paint.
///
/// The context deliberately contains no mounted identity, surface origin,
/// raster scale, renderer/backend object, resource provider, semantic data, or
/// publication history.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintContributionContext {
    local_size: LogicalSize,
    computed_style: ComputedStyle,
}

impl PaintContributionContext {
    /// Returns the owner's final local logical size.
    #[must_use]
    pub const fn local_size(&self) -> LogicalSize {
        self.local_size
    }

    /// Returns the owner's resolved style facts.
    #[must_use]
    pub const fn computed_style(&self) -> &ComputedStyle {
        &self.computed_style
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(local_size: LogicalSize, computed_style: ComputedStyle) -> Self {
        Self {
            local_size,
            computed_style,
        }
    }
}

/// Ordered immutable paint fragment authored in one widget's local logical space.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintContribution {
    items: Vec<PaintContributionItem>,
}

impl PaintContribution {
    /// Empty contribution.
    #[must_use]
    pub const fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Creates one contribution from already validated items in local order.
    #[must_use]
    pub const fn new(items: Vec<PaintContributionItem>) -> Self {
        Self { items }
    }

    /// Creates a one-item contribution.
    #[must_use]
    pub fn single(item: PaintContributionItem) -> Self {
        Self { items: vec![item] }
    }

    /// Returns contribution items in exact authored order.
    #[must_use]
    pub const fn items(&self) -> &[PaintContributionItem] {
        self.items.as_slice()
    }

    /// Returns whether this widget contributes no paint.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Exact runtime-resolved source rectangle in intrinsic image-pixel space.
///
/// This value is publication geometry, not authored fit/crop policy and not a
/// renderer UV or raster/device coordinate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedImageSourceRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl ResolvedImageSourceRect {
    /// Runtime bridge for one finite non-negative source rectangle.
    ///
    /// The runtime derives these values only from an already-validated image
    /// descriptor and its exact intrinsic extent.
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_new(x: f32, y: f32, width: f32, height: f32) -> Option<Self> {
        if ![x, y, width, height].into_iter().all(f32::is_finite)
            || x < 0.0
            || y < 0.0
            || width < 0.0
            || height < 0.0
        {
            return None;
        }
        Some(Self {
            x,
            y,
            width,
            height,
        })
    }

    /// Returns intrinsic-pixel source x.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    /// Returns intrinsic-pixel source y.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }

    /// Returns intrinsic-pixel source width.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.width
    }

    /// Returns intrinsic-pixel source height.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.height
    }
}

/// One exact runtime-resolved image source/destination mapping patch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedImagePatch {
    source: ResolvedImageSourceRect,
    destination: LogicalRect,
}

impl ResolvedImagePatch {
    /// Runtime bridge for one already-resolved image patch.
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(source: ResolvedImageSourceRect, destination: LogicalRect) -> Self {
        Self {
            source,
            destination,
        }
    }

    /// Returns the exact intrinsic-pixel source rectangle.
    #[must_use]
    pub const fn source(self) -> ResolvedImageSourceRect {
        self.source
    }

    /// Returns the exact owner-local logical destination rectangle.
    #[must_use]
    pub const fn destination(self) -> LogicalRect {
        self.destination
    }
}

/// Runtime-resolved image publication facts.
///
/// Fit/crop/alignment/nine-slice policy is deliberately absent. The complete
/// `ResourceRef`, exact descriptor intrinsic extent, and final source/destination
/// patches are sufficient for disposable renderer realization.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedImagePrimitive {
    resource: ResourceRef,
    intrinsic_size: ImageIntrinsicSize,
    patches: Vec<ResolvedImagePatch>,
}

impl ResolvedImagePrimitive {
    /// Runtime bridge for exact resolved image publication facts.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceKindMismatch`] when `resource` is not image-kind.
    #[doc(hidden)]
    pub fn __runtime_new(
        resource: ResourceRef,
        intrinsic_size: ImageIntrinsicSize,
        patches: Vec<ResolvedImagePatch>,
    ) -> Result<Self, ResourceKindMismatch> {
        if resource.kind() != ResourceKind::Image {
            return Err(ResourceKindMismatch::new(
                ResourceKind::Image,
                resource.kind(),
            ));
        }
        Ok(Self {
            resource,
            intrinsic_size,
            patches,
        })
    }

    /// Returns the unchanged complete opaque image-resource reference.
    #[must_use]
    pub const fn resource_ref(&self) -> &ResourceRef {
        &self.resource
    }

    /// Returns the exact descriptor intrinsic extent retained for provider correlation.
    #[must_use]
    pub const fn intrinsic_size(&self) -> ImageIntrinsicSize {
        self.intrinsic_size
    }

    /// Returns final resolved source/destination patches in deterministic order.
    #[must_use]
    pub const fn patches(&self) -> &[ResolvedImagePatch] {
        self.patches.as_slice()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ImagePrimitivePhase {
    Authored(ImagePaintDescriptor),
    Resolved(ResolvedImagePrimitive),
}

/// Image paint primitive with an explicit authored-to-publication phase boundary.
///
/// Owner-local contributions carry an [`ImagePaintDescriptor`]. The runtime
/// replaces that authored policy with [`ResolvedImagePrimitive`] before creating
/// a `PaintSceneItem`; renderer-visible image geometry therefore never requires
/// fit/crop/nine-slice interpretation.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePrimitive {
    phase: ImagePrimitivePhase,
}

impl ImagePrimitive {
    const fn authored(descriptor: ImagePaintDescriptor) -> Self {
        Self {
            phase: ImagePrimitivePhase::Authored(descriptor),
        }
    }

    /// Runtime bridge for replacing authored image policy with resolved publication facts.
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_resolved(resolved: ResolvedImagePrimitive) -> Self {
        Self {
            phase: ImagePrimitivePhase::Resolved(resolved),
        }
    }

    /// Returns the complete opaque image-resource reference in either phase.
    #[must_use]
    pub const fn resource_ref(&self) -> &ResourceRef {
        match &self.phase {
            ImagePrimitivePhase::Authored(descriptor) => descriptor.image().resource_ref(),
            ImagePrimitivePhase::Resolved(resolved) => resolved.resource_ref(),
        }
    }

    /// Returns owner-authored image policy when this value is still contribution-local.
    #[must_use]
    pub const fn authored_descriptor(&self) -> Option<&ImagePaintDescriptor> {
        match &self.phase {
            ImagePrimitivePhase::Authored(descriptor) => Some(descriptor),
            ImagePrimitivePhase::Resolved(_) => None,
        }
    }

    /// Returns runtime-resolved image publication facts when this value is scene-ready.
    #[must_use]
    pub const fn resolved(&self) -> Option<&ResolvedImagePrimitive> {
        match &self.phase {
            ImagePrimitivePhase::Authored(_) => None,
            ImagePrimitivePhase::Resolved(resolved) => Some(resolved),
        }
    }
}

/// One validated shaped-text-run paint primitive.
///
/// `origin` maps resource-local logical `(0, 0)` into the owner's local logical
/// coordinates. Glyph geometry remains resource-owned; `foreground` is ordinary
/// scene-owned literal color and is intentionally outside resource identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedTextRunPrimitive {
    resource: ResourceRef,
    origin: LogicalPoint,
    foreground: Color,
}

impl ShapedTextRunPrimitive {
    /// Creates a shaped-run primitive from a shaped-text-run resource reference.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceKindMismatch`] when `resource` has another kind.
    pub fn new(
        resource: ResourceRef,
        origin: LogicalPoint,
        foreground: Color,
    ) -> Result<Self, ResourceKindMismatch> {
        if resource.kind() != ResourceKind::ShapedTextRun {
            return Err(ResourceKindMismatch::new(
                ResourceKind::ShapedTextRun,
                resource.kind(),
            ));
        }
        Ok(Self {
            resource,
            origin,
            foreground,
        })
    }

    /// Returns the complete opaque shaped-run resource reference.
    #[must_use]
    pub const fn resource_ref(&self) -> &ResourceRef {
        &self.resource
    }

    /// Returns the finite owner-local placement of resource-local `(0, 0)`.
    #[must_use]
    pub const fn origin(&self) -> LogicalPoint {
        self.origin
    }

    /// Returns the ordinary literal core foreground color.
    #[must_use]
    pub const fn foreground(&self) -> Color {
        self.foreground
    }
}

/// One owner-local renderer-neutral paint item.
///
/// Every item is self-contained: primitive, owner-local transform, conjunctive
/// clips, validated opacity, and snapshot-local layer are explicit values rather
/// than push/pop command state.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintContributionItem {
    primitive: PaintPrimitive,
    local_transform: LogicalTransform,
    clips: Vec<ContributionClip>,
    opacity: SceneOpacity,
    layer: SceneLayer,
}

impl PaintContributionItem {
    const fn from_primitive(primitive: PaintPrimitive) -> Self {
        Self {
            primitive,
            local_transform: LogicalTransform::IDENTITY,
            clips: Vec::new(),
            opacity: SceneOpacity::OPAQUE,
            layer: SceneLayer::ZERO,
        }
    }

    /// Creates one generic filled logical shape.
    #[must_use]
    pub const fn fill(shape: SceneShape, brush: Brush) -> Self {
        Self::from_primitive(PaintPrimitive::Fill { shape, brush })
    }

    /// Creates one generic centered logical shape stroke.
    ///
    /// [`StrokeStyle`] owns the complete initial cap/join/miter contract. A zero
    /// width remains literal no-coverage semantics and is never a backend hairline.
    #[must_use]
    pub const fn stroke(shape: SceneShape, brush: Brush, style: StrokeStyle) -> Self {
        Self::from_primitive(PaintPrimitive::Stroke {
            shape,
            brush,
            style,
        })
    }

    /// Creates one owner-local image item from complete validated image paint policy.
    #[must_use]
    pub const fn image(descriptor: ImagePaintDescriptor) -> Self {
        Self::from_primitive(PaintPrimitive::Image(ImagePrimitive::authored(descriptor)))
    }

    /// Creates a shaped-text-run item with exact owner-local origin and literal foreground.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceKindMismatch`] when `resource` is not shaped-run-kind.
    pub fn shaped_text_run(
        resource: ResourceRef,
        origin: LogicalPoint,
        foreground: Color,
    ) -> Result<Self, ResourceKindMismatch> {
        ShapedTextRunPrimitive::new(resource, origin, foreground)
            .map(|run| Self::from_primitive(PaintPrimitive::ShapedTextRun(run)))
    }

    /// Replaces the item's primitive-local to owner-local transform.
    #[must_use]
    pub const fn with_transform(mut self, transform: LogicalTransform) -> Self {
        self.local_transform = transform;
        self
    }

    /// Appends one conjunctive owner-local clip.
    #[must_use]
    pub fn with_clip(mut self, clip: ContributionClip) -> Self {
        self.clips.push(clip);
        self
    }

    /// Replaces item opacity.
    #[must_use]
    pub const fn with_opacity(mut self, opacity: SceneOpacity) -> Self {
        self.opacity = opacity;
        self
    }

    /// Replaces snapshot-local ordering layer.
    #[must_use]
    pub const fn with_layer(mut self, layer: SceneLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Returns the renderer-neutral primitive.
    #[must_use]
    pub const fn primitive(&self) -> &PaintPrimitive {
        &self.primitive
    }

    /// Returns primitive-local to owner-local transform.
    #[must_use]
    pub const fn local_transform(&self) -> LogicalTransform {
        self.local_transform
    }

    /// Returns conjunctive clips in authored order.
    #[must_use]
    pub const fn clips(&self) -> &[ContributionClip] {
        self.clips.as_slice()
    }

    /// Returns validated item opacity.
    #[must_use]
    pub const fn opacity(&self) -> SceneOpacity {
        self.opacity
    }

    /// Returns snapshot-local ordering layer.
    #[must_use]
    pub const fn layer(&self) -> SceneLayer {
        self.layer
    }
}

/// Minimum renderer-neutral paint primitive vocabulary.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PaintPrimitive {
    /// Generic logical shape filled by one `RunenUI` brush.
    Fill { shape: SceneShape, brush: Brush },
    /// Generic logical shape stroked by one `RunenUI` brush and centered stroke style.
    Stroke {
        shape: SceneShape,
        brush: Brush,
        style: StrokeStyle,
    },
    /// Image contribution/publication value with an explicit authored/resolved phase boundary.
    Image(ImagePrimitive),
    /// Shaped resource whose local origin is placed at one finite logical point.
    ShapedTextRun(ShapedTextRunPrimitive),
}

impl PaintPrimitive {
    /// Returns generic shape geometry for fill/stroke primitives.
    #[must_use]
    pub const fn shape(&self) -> Option<&SceneShape> {
        match self {
            Self::Fill { shape, .. } | Self::Stroke { shape, .. } => Some(shape),
            Self::Image(_) | Self::ShapedTextRun(_) => None,
        }
    }

    /// Returns the `RunenUI` brush for generic fill/stroke primitives.
    #[must_use]
    pub const fn brush(&self) -> Option<&Brush> {
        match self {
            Self::Fill { brush, .. } | Self::Stroke { brush, .. } => Some(brush),
            Self::Image(_) | Self::ShapedTextRun(_) => None,
        }
    }

    /// Returns centered stroke policy when this is a stroke primitive.
    #[must_use]
    pub const fn stroke_style(&self) -> Option<StrokeStyle> {
        match self {
            Self::Stroke { style, .. } => Some(*style),
            Self::Fill { .. } | Self::Image(_) | Self::ShapedTextRun(_) => None,
        }
    }

    /// Returns the complete opaque resource reference for resource-backed primitives.
    #[must_use]
    pub const fn resource_ref(&self) -> Option<&ResourceRef> {
        match self {
            Self::Image(image) => Some(image.resource_ref()),
            Self::ShapedTextRun(run) => Some(run.resource_ref()),
            Self::Fill { .. } | Self::Stroke { .. } => None,
        }
    }

    /// Returns image-specific authored/resolved facts when this is an image primitive.
    #[must_use]
    pub const fn as_image(&self) -> Option<&ImagePrimitive> {
        match self {
            Self::Image(image) => Some(image),
            Self::Fill { .. } | Self::Stroke { .. } | Self::ShapedTextRun(_) => None,
        }
    }

    /// Returns shaped-run-specific placement/color facts when this is a shaped run.
    #[must_use]
    pub const fn as_shaped_text_run(&self) -> Option<&ShapedTextRunPrimitive> {
        match self {
            Self::ShapedTextRun(run) => Some(run),
            Self::Fill { .. } | Self::Stroke { .. } | Self::Image(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PaintContribution, PaintContributionItem, PaintPrimitive};
    use crate::{
        Brush, Color, ContributionClip, ImageDescriptor, ImageIntrinsicSize, ImageMapping,
        ImagePaintDescriptor, LogicalLength, LogicalPoint, LogicalRect, LogicalTransform,
        ResourceKind, ResourceKindMismatch, ResourceRef, SceneLayer, SceneOpacity, SceneShape,
        StrokeStyle,
    };

    #[test]
    fn contribution_preserves_generic_shape_brush_stroke_and_order() {
        let first_rect = LogicalRect::try_new(0.0, 0.0, 10.0, 20.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let second_rect = LogicalRect::try_new(1.0, 2.0, 3.0, 4.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let stroke = StrokeStyle::new(
            LogicalLength::new(2.0).unwrap_or_else(|_| unreachable!("test stroke width is valid")),
        );
        let first_brush = Brush::solid(Color::rgba(1, 2, 3, 4));
        let second_brush = Brush::solid(Color::rgba(5, 6, 7, 8));
        let contribution = PaintContribution::new(vec![
            PaintContributionItem::fill(SceneShape::rect(first_rect), first_brush.clone()),
            PaintContributionItem::stroke(
                SceneShape::rect(second_rect),
                second_brush.clone(),
                stroke,
            ),
        ]);

        assert_eq!(contribution.items().len(), 2);
        assert!(matches!(
            contribution.items()[0].primitive(),
            PaintPrimitive::Fill { shape: SceneShape::Rect(rect), brush }
                if *rect == first_rect && brush == &first_brush
        ));
        assert!(matches!(
            contribution.items()[1].primitive(),
            PaintPrimitive::Stroke { shape: SceneShape::Rect(rect), brush, style }
                if *rect == second_rect && brush == &second_brush && *style == stroke
        ));
        assert_eq!(
            contribution.items()[0].primitive().shape(),
            Some(&SceneShape::rect(first_rect))
        );
        assert_eq!(
            contribution.items()[0].primitive().brush(),
            Some(&first_brush)
        );
        assert_eq!(
            contribution.items()[1].primitive().stroke_style(),
            Some(stroke)
        );
    }

    #[test]
    fn zero_width_stroke_remains_literal_zero() {
        let rect = LogicalRect::try_new(0.0, 0.0, 1.0, 1.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let style = StrokeStyle::new(LogicalLength::ZERO);
        let item = PaintContributionItem::stroke(
            SceneShape::rect(rect),
            Brush::solid(Color::BLACK),
            style,
        );
        assert_eq!(item.primitive().stroke_style(), Some(style));
    }

    #[test]
    fn resource_primitives_preserve_authored_image_policy_and_shaped_run_facts() {
        let rect = LogicalRect::try_new(2.0, 3.0, 40.0, 50.0)
            .unwrap_or_else(|_| unreachable!("test destination is valid"));
        let origin =
            LogicalPoint::new(4.0, 7.0).unwrap_or_else(|_| unreachable!("test origin is finite"));
        let image_ref = ResourceRef::new(ResourceKind::Image);
        let shaped_ref = ResourceRef::new(ResourceKind::ShapedTextRun);
        let image_descriptor = ImageDescriptor::new(
            image_ref.clone(),
            ImageIntrinsicSize::new(40, 50)
                .unwrap_or_else(|| unreachable!("test image extent is non-zero")),
        )
        .unwrap_or_else(|_| unreachable!("image ref has image kind"));
        let image_paint = ImagePaintDescriptor::new(image_descriptor, rect, ImageMapping::default())
            .unwrap_or_else(|_| unreachable!("test image mapping is valid"));

        let image = PaintContributionItem::image(image_paint.clone());
        let run = PaintContributionItem::shaped_text_run(
            shaped_ref.clone(),
            origin,
            Color::rgba(1, 2, 3, 4),
        )
        .unwrap_or_else(|_| unreachable!("shaped ref has shaped-run kind"));

        assert_eq!(image.primitive().resource_ref(), Some(&image_ref));
        assert_eq!(
            image
                .primitive()
                .as_image()
                .and_then(super::ImagePrimitive::authored_descriptor),
            Some(&image_paint)
        );
        assert!(
            image
                .primitive()
                .as_image()
                .and_then(super::ImagePrimitive::resolved)
                .is_none()
        );
        assert_eq!(run.primitive().resource_ref(), Some(&shaped_ref));
        assert_eq!(
            run.primitive()
                .as_shaped_text_run()
                .map(super::ShapedTextRunPrimitive::origin),
            Some(origin)
        );
        assert_eq!(
            run.primitive()
                .as_shaped_text_run()
                .map(super::ShapedTextRunPrimitive::foreground),
            Some(Color::rgba(1, 2, 3, 4))
        );

        let Err(wrong_run) =
            PaintContributionItem::shaped_text_run(image_ref, origin, Color::BLACK)
        else {
            unreachable!("image refs cannot become shaped-run primitives");
        };
        assert_eq!(
            wrong_run,
            ResourceKindMismatch::new(ResourceKind::ShapedTextRun, ResourceKind::Image)
        );
    }

    #[test]
    fn item_composition_defaults_and_explicit_values_are_self_contained() {
        let rect = LogicalRect::try_new(0.0, 0.0, 4.0, 5.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let default_item =
            PaintContributionItem::fill(SceneShape::rect(rect), Brush::solid(Color::WHITE));
        assert_eq!(default_item.local_transform(), LogicalTransform::IDENTITY);
        assert!(default_item.clips().is_empty());
        assert_eq!(default_item.opacity(), SceneOpacity::OPAQUE);
        assert_eq!(default_item.layer(), SceneLayer::ZERO);

        let transform = LogicalTransform::translation(3.0, 7.0)
            .unwrap_or_else(|_| unreachable!("test transform is valid"));
        let opacity =
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("test opacity is valid"));
        let clip = ContributionClip::identity(SceneShape::rect(rect));
        let item = default_item
            .with_transform(transform)
            .with_clip(clip.clone())
            .with_opacity(opacity)
            .with_layer(SceneLayer::new(-2));
        assert_eq!(item.local_transform(), transform);
        assert_eq!(item.clips(), &[clip]);
        assert_eq!(item.opacity(), opacity);
        assert_eq!(item.layer(), SceneLayer::new(-2));
    }
}
