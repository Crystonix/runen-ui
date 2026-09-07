//! RunenUI-owned structural path vocabulary.

use core::{error::Error, fmt};
use std::sync::Arc;

use crate::{LogicalPoint, LogicalRect};

/// Fill rule applied to logically closed path contours.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PathFillRule {
    /// Non-zero winding fill.
    #[default]
    NonZero,
    /// Even-odd parity fill.
    EvenOdd,
}

/// One immutable authored path verb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathVerb {
    /// Starts a new contour at the finite point.
    MoveTo(LogicalPoint),
    /// Adds one straight segment to the finite point.
    LineTo(LogicalPoint),
    /// Adds one quadratic Bézier segment.
    QuadraticTo {
        /// Quadratic control point.
        control: LogicalPoint,
        /// Segment endpoint.
        to: LogicalPoint,
    },
    /// Adds one cubic Bézier segment.
    CubicTo {
        /// First cubic control point.
        control1: LogicalPoint,
        /// Second cubic control point.
        control2: LogicalPoint,
        /// Segment endpoint.
        to: LogicalPoint,
    },
    /// Explicitly closes the current segment-bearing contour.
    Close,
}

/// Validation failure for one structural path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenePathError {
    /// A segment was authored before the first `move`.
    SegmentWithoutContour,
    /// `close` was authored before the contour had a segment.
    CloseWithoutSegment,
    /// The current contour was already explicitly closed.
    AlreadyClosed,
    /// A segment followed an explicit close without a new move.
    SegmentAfterClose,
}

impl fmt::Display for ScenePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SegmentWithoutContour => "path segment requires a preceding move",
            Self::CloseWithoutSegment => "path close requires a segment-bearing open contour",
            Self::AlreadyClosed => "path contour is already closed",
            Self::SegmentAfterClose => "path segment after close requires a new move",
        })
    }
}

impl Error for ScenePathError {}

/// Immutable validated RunenUI path content.
///
/// Identity/equality is structural path content plus fill rule. Shared storage is
/// an implementation detail and never participates in scene identity.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenePath {
    verbs: Arc<[PathVerb]>,
    fill_rule: PathFillRule,
    logical_bounds: Option<LogicalRect>,
}

impl ScenePath {
    /// Validates and freezes authored path content.
    ///
    /// Move-only contours are accepted but have no coverage. Open
    /// segment-bearing contours remain structurally open: ADR 0011's synthetic
    /// closing edge exists only during fill evaluation and is not inserted here.
    ///
    /// # Errors
    ///
    /// Returns [`ScenePathError`] for malformed contour ordering.
    pub fn new(
        verbs: impl Into<Vec<PathVerb>>,
        fill_rule: PathFillRule,
    ) -> Result<Self, ScenePathError> {
        let verbs = verbs.into();
        validate_verbs(&verbs)?;
        let logical_bounds = path_bounds(&verbs);
        Ok(Self {
            verbs: Arc::from(verbs),
            fill_rule,
            logical_bounds,
        })
    }

    /// Returns exact authored verbs in stable order.
    #[must_use]
    pub fn verbs(&self) -> &[PathVerb] {
        self.verbs.as_ref()
    }

    /// Returns the authored fill rule.
    #[must_use]
    pub const fn fill_rule(&self) -> PathFillRule {
        self.fill_rule
    }

    /// Returns deterministic tight segment bounds when the path has at least one
    /// authored segment. Move-only/empty paths return `None`.
    #[must_use]
    pub const fn logical_bounds(&self) -> Option<LogicalRect> {
        self.logical_bounds
    }

    /// Returns whether no authored segment contributes geometric coverage.
    #[must_use]
    pub const fn is_coverage_empty(&self) -> bool {
        self.logical_bounds.is_none()
    }
}

fn validate_verbs(verbs: &[PathVerb]) -> Result<(), ScenePathError> {
    let mut has_contour = false;
    let mut has_segment = false;
    let mut closed = false;

    for verb in verbs {
        match verb {
            PathVerb::MoveTo(_) => {
                has_contour = true;
                has_segment = false;
                closed = false;
            }
            PathVerb::LineTo(_) | PathVerb::QuadraticTo { .. } | PathVerb::CubicTo { .. } => {
                if !has_contour {
                    return Err(ScenePathError::SegmentWithoutContour);
                }
                if closed {
                    return Err(ScenePathError::SegmentAfterClose);
                }
                has_segment = true;
            }
            PathVerb::Close => {
                if !has_contour || !has_segment {
                    return Err(ScenePathError::CloseWithoutSegment);
                }
                if closed {
                    return Err(ScenePathError::AlreadyClosed);
                }
                closed = true;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds {
    fn point(point: LogicalPoint) -> Self {
        let x = f64::from(point.x());
        let y = f64::from(point.y());
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    fn include(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn rect(self) -> Option<LogicalRect> {
        LogicalRect::try_new(
            self.min_x as f32,
            self.min_y as f32,
            (self.max_x - self.min_x) as f32,
            (self.max_y - self.min_y) as f32,
        )
        .ok()
    }
}

fn path_bounds(verbs: &[PathVerb]) -> Option<LogicalRect> {
    let mut current = None;
    let mut first = None;
    let mut bounds: Option<Bounds> = None;

    for verb in verbs {
        match *verb {
            PathVerb::MoveTo(point) => {
                current = Some(point);
                first = Some(point);
            }
            PathVerb::LineTo(to) => {
                let from = current?;
                include_line(&mut bounds, from, to);
                current = Some(to);
            }
            PathVerb::QuadraticTo { control, to } => {
                let from = current?;
                include_quadratic(&mut bounds, from, control, to);
                current = Some(to);
            }
            PathVerb::CubicTo {
                control1,
                control2,
                to,
            } => {
                let from = current?;
                include_cubic(&mut bounds, from, control1, control2, to);
                current = Some(to);
            }
            PathVerb::Close => {
                let from = current?;
                let to = first?;
                include_line(&mut bounds, from, to);
                current = Some(to);
            }
        }
    }
    bounds.and_then(Bounds::rect)
}

fn ensure_bounds(bounds: &mut Option<Bounds>, point: LogicalPoint) -> &mut Bounds {
    bounds.get_or_insert_with(|| Bounds::point(point))
}

fn include_line(bounds: &mut Option<Bounds>, from: LogicalPoint, to: LogicalPoint) {
    let bounds = ensure_bounds(bounds, from);
    bounds.include(f64::from(to.x()), f64::from(to.y()));
}

fn include_quadratic(
    bounds: &mut Option<Bounds>,
    from: LogicalPoint,
    control: LogicalPoint,
    to: LogicalPoint,
) {
    include_line(bounds, from, to);
    let bounds = ensure_bounds(bounds, from);
    for axis in 0..2 {
        let (p0, p1, p2) = match axis {
            0 => (f64::from(from.x()), f64::from(control.x()), f64::from(to.x())),
            _ => (f64::from(from.y()), f64::from(control.y()), f64::from(to.y())),
        };
        let denominator = p0 - 2.0 * p1 + p2;
        if denominator != 0.0 {
            let t = (p0 - p1) / denominator;
            if (0.0..1.0).contains(&t) {
                let x = quadratic(
                    f64::from(from.x()),
                    f64::from(control.x()),
                    f64::from(to.x()),
                    t,
                );
                let y = quadratic(
                    f64::from(from.y()),
                    f64::from(control.y()),
                    f64::from(to.y()),
                    t,
                );
                bounds.include(x, y);
            }
        }
    }
}

fn include_cubic(
    bounds: &mut Option<Bounds>,
    from: LogicalPoint,
    control1: LogicalPoint,
    control2: LogicalPoint,
    to: LogicalPoint,
) {
    include_line(bounds, from, to);
    let mut roots = [0.0_f64; 2];
    for axis in 0..2 {
        let (p0, p1, p2, p3) = match axis {
            0 => (
                f64::from(from.x()),
                f64::from(control1.x()),
                f64::from(control2.x()),
                f64::from(to.x()),
            ),
            _ => (
                f64::from(from.y()),
                f64::from(control1.y()),
                f64::from(control2.y()),
                f64::from(to.y()),
            ),
        };
        let a = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
        let b = 2.0 * (p0 - 2.0 * p1 + p2);
        let c = p1 - p0;
        let count = quadratic_roots(3.0 * a, b * 1.5, 3.0 * c, &mut roots);
        for &t in roots[..count].iter() {
            if (0.0..1.0).contains(&t) {
                let x = cubic(
                    f64::from(from.x()),
                    f64::from(control1.x()),
                    f64::from(control2.x()),
                    f64::from(to.x()),
                    t,
                );
                let y = cubic(
                    f64::from(from.y()),
                    f64::from(control1.y()),
                    f64::from(control2.y()),
                    f64::from(to.y()),
                    t,
                );
                ensure_bounds(bounds, from).include(x, y);
            }
        }
    }
}

fn quadratic_roots(a: f64, b: f64, c: f64, roots: &mut [f64; 2]) -> usize {
    if a == 0.0 {
        if b == 0.0 {
            return 0;
        }
        roots[0] = -c / b;
        return 1;
    }
    let discriminant = b.mul_add(b, -4.0 * a * c);
    if discriminant < 0.0 {
        return 0;
    }
    if discriminant == 0.0 {
        roots[0] = -b / (2.0 * a);
        return 1;
    }
    let sqrt = discriminant.sqrt();
    let q = -0.5 * (b + sqrt.copysign(b));
    roots[0] = q / a;
    roots[1] = c / q;
    2
}

fn quadratic(p0: f64, p1: f64, p2: f64, t: f64) -> f64 {
    let one_minus = 1.0 - t;
    one_minus * one_minus * p0 + 2.0 * one_minus * t * p1 + t * t * p2
}

fn cubic(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let one_minus = 1.0 - t;
    one_minus * one_minus * one_minus * p0
        + 3.0 * one_minus * one_minus * t * p1
        + 3.0 * one_minus * t * t * p2
        + t * t * t * p3
}

#[cfg(test)]
mod tests {
    use super::{PathFillRule, PathVerb, ScenePath, ScenePathError};
    use crate::LogicalPoint;

    fn point(x: f32, y: f32) -> LogicalPoint {
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    #[test]
    fn malformed_contours_reject_without_dependency_recovery() {
        assert_eq!(
            ScenePath::new(vec![PathVerb::LineTo(point(1.0, 1.0))], PathFillRule::NonZero),
            Err(ScenePathError::SegmentWithoutContour)
        );
        assert_eq!(
            ScenePath::new(vec![PathVerb::MoveTo(point(0.0, 0.0)), PathVerb::Close], PathFillRule::NonZero),
            Err(ScenePathError::CloseWithoutSegment)
        );
        assert_eq!(
            ScenePath::new(
                vec![
                    PathVerb::MoveTo(point(0.0, 0.0)),
                    PathVerb::LineTo(point(1.0, 0.0)),
                    PathVerb::Close,
                    PathVerb::LineTo(point(2.0, 0.0)),
                ],
                PathFillRule::NonZero,
            ),
            Err(ScenePathError::SegmentAfterClose)
        );
    }

    #[test]
    fn structural_identity_does_not_depend_on_shared_allocation() {
        let verbs = vec![
            PathVerb::MoveTo(point(0.0, 0.0)),
            PathVerb::QuadraticTo {
                control: point(5.0, 10.0),
                to: point(10.0, 0.0),
            },
        ];
        let a = ScenePath::new(verbs.clone(), PathFillRule::EvenOdd)
            .unwrap_or_else(|_| unreachable!("test path is valid"));
        let b = ScenePath::new(verbs, PathFillRule::EvenOdd)
            .unwrap_or_else(|_| unreachable!("test path is valid"));
        assert_eq!(a, b);
        assert_eq!(a.verbs(), b.verbs());
    }

    #[test]
    fn tight_bounds_include_quadratic_extrema_and_ignore_move_only_contours() {
        let empty = ScenePath::new(
            vec![PathVerb::MoveTo(point(20.0, 30.0))],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("move-only path is valid"));
        assert!(empty.logical_bounds().is_none());

        let path = ScenePath::new(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::QuadraticTo {
                    control: point(5.0, 10.0),
                    to: point(10.0, 0.0),
                },
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("test path is valid"));
        let bounds = path
            .logical_bounds()
            .unwrap_or_else(|| unreachable!("segment-bearing path has bounds"));
        assert_eq!((bounds.x(), bounds.y(), bounds.width(), bounds.height()), (0.0, 0.0, 10.0, 5.0));
    }
}
