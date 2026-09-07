//! Renderer-neutral M9 visual descriptor vocabulary.

use core::{error::Error, fmt, num::NonZeroU32};
use std::sync::Arc;

use crate::{
    Color, LogicalLength, LogicalPoint, LogicalRect, ResourceKind, ResourceKindMismatch, ResourceRef,
};

/// Validation failure for normalized unit-interval values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitIntervalError {
    /// The value was NaN or infinite.
    NotFinite,
    /// The finite value was outside `[0, 1]`.
    OutOfRange,
}

impl fmt::Display for UnitIntervalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFinite => "normalized value must be finite",
            Self::OutOfRange => "normalized value must be within [0, 1]",
        })
    }
}

impl Error for UnitIntervalError {}

/// Finite normalized value in the closed `[0, 1]` range.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct UnitInterval(f32);

impl UnitInterval {
    /// Start of the normalized interval.
    pub const ZERO: Self = Self(0.0);
    /// Midpoint of the normalized interval.
    pub const HALF: Self = Self(0.5);
    /// End of the normalized interval.
    pub const ONE: Self = Self(1.0);

    /// Validates one unit-interval value.
    ///
    /// # Errors
    ///
    /// Returns [`UnitIntervalError`] when non-finite or outside `[0, 1]`.
    pub const fn new(value: f32) -> Result<Self, UnitIntervalError> {
        if value.is_nan() || value == f32::INFINITY || value == f32::NEG_INFINITY {
            Err(UnitIntervalError::NotFinite)
        } else if value < 0.0 || value > 1.0 {
            Err(UnitIntervalError::OutOfRange)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the validated scalar.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// One validated gradient stop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    offset: UnitInterval,
    color: Color,
}

impl GradientStop {
    /// Creates one already-validated stop.
    #[must_use]
    pub const fn new(offset: UnitInterval, color: Color) -> Self {
        Self { offset, color }
    }

    /// Returns the normalized stop offset.
    #[must_use]
    pub const fn offset(self) -> UnitInterval {
        self.offset
    }

    /// Returns the straight-alpha sRGB8 public color.
    #[must_use]
    pub const fn color(self) -> Color {
        self.color
    }
}

/// Validation failure for a complete gradient-stop list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GradientStopsError {
    /// Fewer than two stops were supplied.
    TooFewStops,
    /// Stop offsets were not in stable nondecreasing order.
    DecreasingOffsets,
}

impl fmt::Display for GradientStopsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TooFewStops => "gradient requires at least two stops",
            Self::DecreasingOffsets => "gradient stop offsets must be nondecreasing",
        })
    }
}

impl Error for GradientStopsError {}

/// Immutable validated stable gradient stops.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientStops(Arc<[GradientStop]>);

impl GradientStops {
    /// Validates and freezes gradient stops.
    ///
    /// Equal offsets are accepted in authored order for deterministic hard-stop
    /// semantics; only decreasing offsets reject.
    ///
    /// # Errors
    ///
    /// Returns [`GradientStopsError`] for too few or decreasing stops.
    pub fn new(stops: impl Into<Vec<GradientStop>>) -> Result<Self, GradientStopsError> {
        let stops = stops.into();
        if stops.len() < 2 {
            return Err(GradientStopsError::TooFewStops);
        }
        if stops
            .windows(2)
            .any(|pair| pair[0].offset().get() > pair[1].offset().get())
        {
            return Err(GradientStopsError::DecreasingOffsets);
        }
        Ok(Self(Arc::from(stops)))
    }

    /// Returns exact authored stops in stable order.
    #[must_use]
    pub fn as_slice(&self) -> &[GradientStop] {
        self.0.as_ref()
    }
}

/// Geometry failure for an accepted gradient.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GradientGeometryError {
    /// A linear gradient used equal endpoints.
    EqualLinearEndpoints,
    /// A radial gradient used zero radius.
    ZeroRadialRadius,
}

impl fmt::Display for GradientGeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EqualLinearEndpoints => "linear gradient endpoints must differ",
            Self::ZeroRadialRadius => "radial gradient radius must be positive",
        })
    }
}

impl Error for GradientGeometryError {}

/// Primitive-local linear gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    start: LogicalPoint,
    end: LogicalPoint,
    stops: GradientStops,
}

impl LinearGradient {
    /// Creates one nondegenerate linear gradient.
    ///
    /// # Errors
    ///
    /// Returns [`GradientGeometryError::EqualLinearEndpoints`] when endpoints match.
    pub fn new(
        start: LogicalPoint,
        end: LogicalPoint,
        stops: GradientStops,
    ) -> Result<Self, GradientGeometryError> {
        if start == end {
            return Err(GradientGeometryError::EqualLinearEndpoints);
        }
        Ok(Self { start, end, stops })
    }

    #[must_use]
    pub const fn start(&self) -> LogicalPoint {
        self.start
    }
    #[must_use]
    pub const fn end(&self) -> LogicalPoint {
        self.end
    }
    #[must_use]
    pub const fn stops(&self) -> &GradientStops {
        &self.stops
    }
}

/// Primitive-local concentric radial gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradient {
    center: LogicalPoint,
    radius: LogicalLength,
    stops: GradientStops,
}

impl RadialGradient {
    /// Creates one positive-radius concentric radial gradient.
    ///
    /// # Errors
    ///
    /// Returns [`GradientGeometryError::ZeroRadialRadius`] for radius zero.
    pub fn new(
        center: LogicalPoint,
        radius: LogicalLength,
        stops: GradientStops,
    ) -> Result<Self, GradientGeometryError> {
        if radius == LogicalLength::ZERO {
            return Err(GradientGeometryError::ZeroRadialRadius);
        }
        Ok(Self {
            center,
            radius,
            stops,
        })
    }

    #[must_use]
    pub const fn center(&self) -> LogicalPoint {
        self.center
    }
    #[must_use]
    pub const fn radius(&self) -> LogicalLength {
        self.radius
    }
    #[must_use]
    pub const fn stops(&self) -> &GradientStops {
        &self.stops
    }
}

/// RunenUI-owned initial brush vocabulary.
#[derive(Clone, Debug, PartialEq)]
pub enum Brush {
    /// Straight-alpha sRGB8 literal solid color.
    Solid(Color),
    /// Nondegenerate primitive-local linear gradient.
    Linear(LinearGradient),
    /// Positive-radius primitive-local concentric radial gradient.
    Radial(RadialGradient),
}

impl Brush {
    /// Creates the trivial solid-brush case.
    #[must_use]
    pub const fn solid(color: Color) -> Self {
        Self::Solid(color)
    }
}

impl From<Color> for Brush {
    fn from(color: Color) -> Self {
        Self::Solid(color)
    }
}

/// Initial neutral stroke cap.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum StrokeCap {
    /// Stop at the endpoint.
    #[default]
    Butt,
    /// Add a semicircular cap.
    Round,
    /// Extend by half stroke width with a square cap.
    Square,
}

/// Initial neutral stroke join.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum StrokeJoin {
    /// Miter, with deterministic bevel fallback above the miter limit.
    #[default]
    Miter,
    /// Bevel join.
    Bevel,
    /// Round join.
    Round,
}

/// Validation failure for one stroke description.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrokeStyleError {
    /// Miter limit was NaN or infinite.
    NonFiniteMiterLimit,
    /// Miter limit was less than the accepted ratio `1`.
    MiterLimitBelowOne,
}

impl fmt::Display for StrokeStyleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFiniteMiterLimit => "stroke miter limit must be finite",
            Self::MiterLimitBelowOne => "stroke miter limit must be at least 1",
        })
    }
}

impl Error for StrokeStyleError {}

/// Centered neutral stroke geometry description.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    width: LogicalLength,
    cap: StrokeCap,
    join: StrokeJoin,
    miter_limit: f32,
}

impl StrokeStyle {
    /// Common centered stroke with butt caps, miter joins, and miter limit `4`.
    #[must_use]
    pub const fn new(width: LogicalLength) -> Self {
        Self {
            width,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            miter_limit: 4.0,
        }
    }

    /// Replaces the cap.
    #[must_use]
    pub const fn with_cap(mut self, cap: StrokeCap) -> Self {
        self.cap = cap;
        self
    }

    /// Replaces the join.
    #[must_use]
    pub const fn with_join(mut self, join: StrokeJoin) -> Self {
        self.join = join;
        self
    }

    /// Validates and replaces the miter-limit ratio.
    ///
    /// # Errors
    ///
    /// Returns [`StrokeStyleError`] for non-finite or sub-one values.
    pub fn with_miter_limit(mut self, miter_limit: f32) -> Result<Self, StrokeStyleError> {
        if !miter_limit.is_finite() {
            return Err(StrokeStyleError::NonFiniteMiterLimit);
        }
        if miter_limit < 1.0 {
            return Err(StrokeStyleError::MiterLimitBelowOne);
        }
        self.miter_limit = miter_limit;
        Ok(self)
    }

    #[must_use]
    pub const fn width(self) -> LogicalLength {
        self.width
    }
    #[must_use]
    pub const fn cap(self) -> StrokeCap {
        self.cap
    }
    #[must_use]
    pub const fn join(self) -> StrokeJoin {
        self.join
    }
    #[must_use]
    pub const fn miter_limit(self) -> f32 {
        self.miter_limit
    }
}

/// Exact non-zero intrinsic image pixel extent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ImageIntrinsicSize {
    width: NonZeroU32,
    height: NonZeroU32,
}

impl ImageIntrinsicSize {
    /// Creates one exact non-zero intrinsic extent.
    ///
    /// Returns `None` when either axis is zero.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Option<Self> {
        let Some(width) = NonZeroU32::new(width) else {
            return None;
        };
        let Some(height) = NonZeroU32::new(height) else {
            return None;
        };
        Some(Self { width, height })
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width.get()
    }
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height.get()
    }
}

/// Validated normalized image crop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageCrop {
    x: UnitInterval,
    y: UnitInterval,
    width: UnitInterval,
    height: UnitInterval,
}

/// Validation failure for one normalized crop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageCropError {
    /// Crop width or height was zero.
    Empty,
    /// Crop extends beyond normalized source extent.
    OutsideSource,
}

impl fmt::Display for ImageCropError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "image crop must have positive width and height",
            Self::OutsideSource => "image crop must remain within normalized source extent",
        })
    }
}

impl Error for ImageCropError {}

impl ImageCrop {
    /// Complete normalized source crop.
    pub const FULL: Self = Self {
        x: UnitInterval::ZERO,
        y: UnitInterval::ZERO,
        width: UnitInterval::ONE,
        height: UnitInterval::ONE,
    };

    /// Validates a normalized positive crop.
    ///
    /// # Errors
    ///
    /// Returns [`ImageCropError`] when empty or outside the source.
    pub fn new(
        x: UnitInterval,
        y: UnitInterval,
        width: UnitInterval,
        height: UnitInterval,
    ) -> Result<Self, ImageCropError> {
        if width == UnitInterval::ZERO || height == UnitInterval::ZERO {
            return Err(ImageCropError::Empty);
        }
        if x.get() + width.get() > 1.0 || y.get() + height.get() > 1.0 {
            return Err(ImageCropError::OutsideSource);
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    #[must_use]
    pub const fn x(self) -> UnitInterval {
        self.x
    }
    #[must_use]
    pub const fn y(self) -> UnitInterval {
        self.y
    }
    #[must_use]
    pub const fn width(self) -> UnitInterval {
        self.width
    }
    #[must_use]
    pub const fn height(self) -> UnitInterval {
        self.height
    }
}

/// Finite normalized image alignment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageAlignment {
    x: UnitInterval,
    y: UnitInterval,
}

impl ImageAlignment {
    /// Center alignment.
    pub const CENTER: Self = Self {
        x: UnitInterval::HALF,
        y: UnitInterval::HALF,
    };

    #[must_use]
    pub const fn new(x: UnitInterval, y: UnitInterval) -> Self {
        Self { x, y }
    }
    #[must_use]
    pub const fn x(self) -> UnitInterval {
        self.x
    }
    #[must_use]
    pub const fn y(self) -> UnitInterval {
        self.y
    }
}

/// Ordinary image fit policy.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ImageFit {
    /// Independently scale each axis to fill destination.
    #[default]
    Fill,
    /// Uniformly fit completely inside destination.
    Contain,
    /// Uniformly cover destination and crop overflow.
    Cover,
    /// Preserve intrinsic logical scale `1`.
    None,
    /// Preserve intrinsic scale when it fits, otherwise contain.
    ScaleDown,
}

/// Complete immutable logical image identity plus intrinsic metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageDescriptor {
    resource: ResourceRef,
    intrinsic_size: ImageIntrinsicSize,
}

impl ImageDescriptor {
    /// Creates one image descriptor from the complete image `ResourceRef`.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceKindMismatch`] when the resource is not image-kind.
    pub fn new(
        resource: ResourceRef,
        intrinsic_size: ImageIntrinsicSize,
    ) -> Result<Self, ResourceKindMismatch> {
        if resource.kind() != ResourceKind::Image {
            return Err(ResourceKindMismatch::new(ResourceKind::Image, resource.kind()));
        }
        Ok(Self {
            resource,
            intrinsic_size,
        })
    }

    #[must_use]
    pub const fn resource_ref(&self) -> &ResourceRef {
        &self.resource
    }
    #[must_use]
    pub const fn intrinsic_size(&self) -> ImageIntrinsicSize {
        self.intrinsic_size
    }
}

/// Four non-negative source-pixel insets for nine-slice mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImageSourceInsets {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

/// Validation failure for source-pixel insets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageSourceInsetsError {
    /// One inset was NaN or infinite.
    NotFinite,
    /// One inset was negative.
    Negative,
}

impl fmt::Display for ImageSourceInsetsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFinite => "image source insets must be finite",
            Self::Negative => "image source insets must be non-negative",
        })
    }
}

impl Error for ImageSourceInsetsError {}

impl ImageSourceInsets {
    /// Validates non-negative finite source-pixel insets.
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Result<Self, ImageSourceInsetsError> {
        if ![top, right, bottom, left].into_iter().all(f32::is_finite) {
            return Err(ImageSourceInsetsError::NotFinite);
        }
        if [top, right, bottom, left].into_iter().any(|value| value < 0.0) {
            return Err(ImageSourceInsetsError::Negative);
        }
        Ok(Self {
            top,
            right,
            bottom,
            left,
        })
    }

    #[must_use]
    pub const fn top(self) -> f32 {
        self.top
    }
    #[must_use]
    pub const fn right(self) -> f32 {
        self.right
    }
    #[must_use]
    pub const fn bottom(self) -> f32 {
        self.bottom
    }
    #[must_use]
    pub const fn left(self) -> f32 {
        self.left
    }
}

/// Four logical destination edge widths for nine-slice mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImageDestinationInsets {
    top: LogicalLength,
    right: LogicalLength,
    bottom: LogicalLength,
    left: LogicalLength,
}

impl ImageDestinationInsets {
    #[must_use]
    pub const fn new(
        top: LogicalLength,
        right: LogicalLength,
        bottom: LogicalLength,
        left: LogicalLength,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
    #[must_use]
    pub const fn top(self) -> LogicalLength {
        self.top
    }
    #[must_use]
    pub const fn right(self) -> LogicalLength {
        self.right
    }
    #[must_use]
    pub const fn bottom(self) -> LogicalLength {
        self.bottom
    }
    #[must_use]
    pub const fn left(self) -> LogicalLength {
        self.left
    }
}

/// Image mapping mode before runtime resolves exact source/destination geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageMapping {
    /// Ordinary crop/fit/alignment mapping.
    Fit {
        crop: ImageCrop,
        alignment: ImageAlignment,
        fit: ImageFit,
    },
    /// Stretched nine-slice mapping. Tiling/repeat is intentionally absent.
    NineSlice {
        source: ImageCrop,
        source_insets: ImageSourceInsets,
        destination_insets: ImageDestinationInsets,
    },
}

impl Default for ImageMapping {
    fn default() -> Self {
        Self::Fit {
            crop: ImageCrop::FULL,
            alignment: ImageAlignment::CENTER,
            fit: ImageFit::Fill,
        }
    }
}

/// Failure while correlating nine-slice source insets with intrinsic source extent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageMappingError {
    /// Opposing source insets overlap within the selected crop.
    OverlappingSourceInsets,
}

impl fmt::Display for ImageMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("nine-slice source insets cannot overlap")
    }
}

impl Error for ImageMappingError {}

/// Complete owner-local image paint descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePaintDescriptor {
    image: ImageDescriptor,
    destination: LogicalRect,
    mapping: ImageMapping,
}

impl ImagePaintDescriptor {
    /// Creates one image paint descriptor and validates nine-slice source insets.
    ///
    /// # Errors
    ///
    /// Returns [`ImageMappingError`] when source insets overlap in the selected crop.
    pub fn new(
        image: ImageDescriptor,
        destination: LogicalRect,
        mapping: ImageMapping,
    ) -> Result<Self, ImageMappingError> {
        if let ImageMapping::NineSlice {
            source,
            source_insets,
            ..
        } = mapping
        {
            let source_width = image.intrinsic_size().width() as f32 * source.width().get();
            let source_height = image.intrinsic_size().height() as f32 * source.height().get();
            if source_insets.left() + source_insets.right() > source_width
                || source_insets.top() + source_insets.bottom() > source_height
            {
                return Err(ImageMappingError::OverlappingSourceInsets);
            }
        }
        Ok(Self {
            image,
            destination,
            mapping,
        })
    }

    #[must_use]
    pub const fn image(&self) -> &ImageDescriptor {
        &self.image
    }
    #[must_use]
    pub const fn destination(&self) -> LogicalRect {
        self.destination
    }
    #[must_use]
    pub const fn mapping(&self) -> ImageMapping {
        self.mapping
    }
}

/// Validation failure for a finite signed logical scalar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NonFiniteVisualScalar;

impl fmt::Display for NonFiniteVisualScalar {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("visual scalar must be finite")
    }
}

impl Error for NonFiniteVisualScalar {}

/// Ordinary renderer-neutral drop shadow description.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropShadow {
    offset_x: f32,
    offset_y: f32,
    sigma: LogicalLength,
    spread: f32,
    color: Color,
}

impl DropShadow {
    /// Creates one finite ordinary drop shadow.
    ///
    /// `sigma` is already finite/non-negative through [`LogicalLength`]. Spread
    /// is signed and therefore validated separately.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteVisualScalar`] for non-finite offset/spread values.
    pub fn new(
        offset_x: f32,
        offset_y: f32,
        sigma: LogicalLength,
        spread: f32,
        color: Color,
    ) -> Result<Self, NonFiniteVisualScalar> {
        if ![offset_x, offset_y, spread].into_iter().all(f32::is_finite) {
            return Err(NonFiniteVisualScalar);
        }
        Ok(Self {
            offset_x,
            offset_y,
            sigma,
            spread,
            color,
        })
    }

    #[must_use]
    pub const fn offset_x(self) -> f32 {
        self.offset_x
    }
    #[must_use]
    pub const fn offset_y(self) -> f32 {
        self.offset_y
    }
    #[must_use]
    pub const fn sigma(self) -> LogicalLength {
        self.sigma
    }
    #[must_use]
    pub const fn spread(self) -> f32 {
        self.spread
    }
    #[must_use]
    pub const fn color(self) -> Color {
        self.color
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Brush, GradientGeometryError, GradientStop, GradientStops, GradientStopsError,
        ImageAlignment, ImageCrop, ImageDescriptor, ImageDestinationInsets, ImageFit,
        ImageIntrinsicSize, ImageMapping, ImageMappingError, ImagePaintDescriptor,
        ImageSourceInsets, LinearGradient, RadialGradient, StrokeStyle, StrokeStyleError,
        UnitInterval,
    };
    use crate::{Color, LogicalLength, LogicalPoint, LogicalRect, ResourceKind, ResourceRef};

    fn point(x: f32, y: f32) -> LogicalPoint {
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    fn stops() -> GradientStops {
        GradientStops::new(vec![
            GradientStop::new(UnitInterval::ZERO, Color::BLACK),
            GradientStop::new(UnitInterval::ONE, Color::WHITE),
        ])
        .unwrap_or_else(|_| unreachable!("test stops are valid"))
    }

    #[test]
    fn gradient_validation_preserves_stable_hard_stops() {
        let half = UnitInterval::new(0.5).unwrap_or_else(|_| unreachable!("half is valid"));
        let hard = GradientStops::new(vec![
            GradientStop::new(UnitInterval::ZERO, Color::BLACK),
            GradientStop::new(half, Color::BLACK),
            GradientStop::new(half, Color::WHITE),
            GradientStop::new(UnitInterval::ONE, Color::WHITE),
        ])
        .unwrap_or_else(|_| unreachable!("nondecreasing hard stops are valid"));
        assert_eq!(hard.as_slice()[1].offset(), hard.as_slice()[2].offset());
        assert_eq!(
            GradientStops::new(vec![GradientStop::new(UnitInterval::ZERO, Color::BLACK)]),
            Err(GradientStopsError::TooFewStops)
        );
    }

    #[test]
    fn degenerate_gradient_geometry_rejects() {
        assert_eq!(
            LinearGradient::new(point(1.0, 1.0), point(1.0, 1.0), stops()),
            Err(GradientGeometryError::EqualLinearEndpoints)
        );
        assert_eq!(
            RadialGradient::new(point(0.0, 0.0), LogicalLength::ZERO, stops()),
            Err(GradientGeometryError::ZeroRadialRadius)
        );
        let solid = Brush::from(Color::BLACK);
        assert_eq!(solid, Brush::Solid(Color::BLACK));
    }

    #[test]
    fn stroke_miter_limit_is_explicit_and_zero_width_remains_zero() {
        let zero = StrokeStyle::new(LogicalLength::ZERO);
        assert_eq!(zero.width(), LogicalLength::ZERO);
        assert_eq!(
            zero.with_miter_limit(0.5),
            Err(StrokeStyleError::MiterLimitBelowOne)
        );
    }

    #[test]
    fn image_descriptor_preserves_resource_identity_across_mapping() {
        let resource = ResourceRef::new(ResourceKind::Image);
        let image = ImageDescriptor::new(
            resource.clone(),
            ImageIntrinsicSize::new(40, 20)
                .unwrap_or_else(|| unreachable!("test intrinsic size is non-zero")),
        )
        .unwrap_or_else(|_| unreachable!("test resource has image kind"));
        let destination = LogicalRect::try_new(0.0, 0.0, 100.0, 100.0)
            .unwrap_or_else(|_| unreachable!("test destination is valid"));
        let paint = ImagePaintDescriptor::new(
            image,
            destination,
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::Contain,
            },
        )
        .unwrap_or_else(|_| unreachable!("test mapping is valid"));
        assert_eq!(paint.image().resource_ref(), &resource);
    }

    #[test]
    fn nine_slice_rejects_overlapping_source_insets() {
        let image = ImageDescriptor::new(
            ResourceRef::new(ResourceKind::Image),
            ImageIntrinsicSize::new(10, 10)
                .unwrap_or_else(|| unreachable!("test intrinsic size is non-zero")),
        )
        .unwrap_or_else(|_| unreachable!("test resource has image kind"));
        let source_insets = ImageSourceInsets::new(0.0, 6.0, 0.0, 6.0)
            .unwrap_or_else(|_| unreachable!("test insets are finite"));
        let destination = LogicalRect::try_new(0.0, 0.0, 20.0, 20.0)
            .unwrap_or_else(|_| unreachable!("test destination is valid"));
        assert_eq!(
            ImagePaintDescriptor::new(
                image,
                destination,
                ImageMapping::NineSlice {
                    source: ImageCrop::FULL,
                    source_insets,
                    destination_insets: ImageDestinationInsets::default(),
                },
            ),
            Err(ImageMappingError::OverlappingSourceInsets)
        );
    }
}
