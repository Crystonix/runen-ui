use core::{error::Error, fmt};
use std::sync::Arc;

use crate::{Color, LogicalLength, LogicalPoint};

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

    /// Returns the primitive-local start point.
    #[must_use]
    pub const fn start(&self) -> LogicalPoint {
        self.start
    }

    /// Returns the primitive-local end point.
    #[must_use]
    pub const fn end(&self) -> LogicalPoint {
        self.end
    }

    /// Returns stable gradient stops.
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

    /// Returns the primitive-local center.
    #[must_use]
    pub const fn center(&self) -> LogicalPoint {
        self.center
    }

    /// Returns the positive logical radius.
    #[must_use]
    pub const fn radius(&self) -> LogicalLength {
        self.radius
    }

    /// Returns stable gradient stops.
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

#[cfg(test)]
mod tests {
    use super::{
        Brush, GradientGeometryError, GradientStop, GradientStops, GradientStopsError,
        LinearGradient, RadialGradient, UnitInterval,
    };
    use crate::{Color, LogicalLength, LogicalPoint};

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
    fn decreasing_gradient_offsets_reject() {
        assert_eq!(
            GradientStops::new(vec![
                GradientStop::new(UnitInterval::ONE, Color::BLACK),
                GradientStop::new(UnitInterval::ZERO, Color::WHITE),
            ]),
            Err(GradientStopsError::DecreasingOffsets)
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
        assert_eq!(Brush::from(Color::BLACK), Brush::Solid(Color::BLACK));
    }
}
