//! Disposable raster realization of ADR 0015 neutral support.
//!
//! The symbolic [`NeutralSupport`] tree remains the geometry authority. This module
//! projects it onto renderer-private physical-pixel masks solely to realize ordinary
//! shadows. Source color, image payload alpha, text/MSDF alpha, item/group opacity,
//! target contents, and device cache state never participate in support generation.

use std::{fmt, sync::Arc};

use runenui_core::{LogicalRect, LogicalTransform, SceneShape};
use runenui_runtime::{RasterScale, SceneClip};

use crate::tessellation::{TessellatedGeometry, tessellate_fill, tessellate_stroke};

use super::support::{NeutralPrimitiveSupport, NeutralShadowFacts, NeutralSupport};

const DISTANCE_INFINITY: f64 = 1.0e30;
const EROSION_SAMPLE_SAFETY: f64 = std::f64::consts::FRAC_1_SQRT_2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MaskLimits {
    max_dimension: u32,
    max_bytes: u64,
}

impl MaskLimits {
    pub(super) const fn new(max_dimension: u32, max_bytes: u64) -> Self {
        Self {
            max_dimension,
            max_bytes,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AlphaMask {
    origin_x: f64,
    origin_y: f64,
    width: u32,
    height: u32,
    alpha: Arc<[u8]>,
}

impl AlphaMask {
    pub(super) const fn origin_x(&self) -> f64 {
        self.origin_x
    }

    pub(super) const fn origin_y(&self) -> f64 {
        self.origin_y
    }

    pub(super) const fn width(&self) -> u32 {
        self.width
    }

    pub(super) const fn height(&self) -> u32 {
        self.height
    }

    pub(super) const fn alpha(&self) -> &Arc<[u8]> {
        &self.alpha
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum MaskError {
    Geometry(String),
    UnresolvedShapedText,
    NonFiniteWorkspace,
    ExtentExceedsDeviceLimit {
        width: u64,
        height: u64,
        max_dimension: u32,
    },
    AllocationExceedsDeviceLimit {
        required_bytes: u64,
        max_bytes: u64,
    },
    AllocationOverflow,
}

impl fmt::Display for MaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry(detail) => write!(formatter, "neutral-support geometry failed: {detail}"),
            Self::UnresolvedShapedText => formatter.write_str(
                "neutral-support shaped text reached raster realization before exact outline resolution",
            ),
            Self::NonFiniteWorkspace => {
                formatter.write_str("neutral-support raster workspace is non-finite")
            }
            Self::ExtentExceedsDeviceLimit {
                width,
                height,
                max_dimension,
            } => write!(
                formatter,
                "neutral-support raster workspace {width}x{height} exceeds device 2D texture limit {max_dimension}"
            ),
            Self::AllocationExceedsDeviceLimit {
                required_bytes,
                max_bytes,
            } => write!(
                formatter,
                "neutral-support raster workspace requires {required_bytes} bytes, exceeding renderer allocation limit {max_bytes}"
            ),
            Self::AllocationOverflow => {
                formatter.write_str("neutral-support raster workspace allocation overflows")
            }
        }
    }
}

pub(super) fn prepare_visual_shadow(
    source: &Arc<NeutralSupport>,
    shadow: NeutralShadowFacts,
    raster_scale: RasterScale,
    limits: MaskLimits,
) -> Result<Option<AlphaMask>, MaskError> {
    let scale = f64::from(raster_scale.get());
    let Some(mut source) = rasterize_support(source, scale, limits)? else {
        return Ok(None);
    };
    source = signed_euclidean_spread(source, shadow.spread() * scale, limits)?;
    if source.is_empty() {
        return Ok(None);
    }
    source.origin_x += shadow.offset_x() * scale;
    source.origin_y += shadow.offset_y() * scale;
    let sigma = shadow.blur_square_half_extent() / 3.0 * scale;
    let blur_radius = shadow.blur_square_half_extent() * scale;
    let blurred = gaussian_blur(source, sigma, blur_radius, limits)?;
    Ok((!blurred.is_empty()).then(|| blurred.into_alpha()))
}

fn rasterize_support(
    support: &Arc<NeutralSupport>,
    scale: f64,
    limits: MaskLimits,
) -> Result<Option<BinaryMask>, MaskError> {
    match support.as_ref() {
        NeutralSupport::Empty => Ok(None),
        NeutralSupport::Primitive(primitive) => rasterize_primitive(primitive, scale, limits),
        NeutralSupport::Union(members) => {
            let masks = members
                .iter()
                .map(|member| rasterize_support(member, scale, limits))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            union_masks(&masks, limits)
        }
        NeutralSupport::Shadow {
            source,
            spread,
            offset_x,
            offset_y,
            blur_square_half_extent,
        } => {
            let Some(source) = rasterize_support(source, scale, limits)? else {
                return Ok(None);
            };
            let mut shadow = signed_euclidean_spread(source, *spread * scale, limits)?;
            if shadow.is_empty() {
                return Ok(None);
            }
            shadow.origin_x += *offset_x * scale;
            shadow.origin_y += *offset_y * scale;
            square_dilate(shadow, *blur_square_half_extent * scale, limits).map(Some)
        }
        NeutralSupport::Clip { source, clips } => {
            let Some(mut source) = rasterize_support(source, scale, limits)? else {
                return Ok(None);
            };
            for clip in clips.iter() {
                intersect_clip(&mut source, clip, scale)?;
                if source.is_empty() {
                    return Ok(None);
                }
            }
            Ok(Some(source))
        }
    }
}

fn rasterize_primitive(
    primitive: &NeutralPrimitiveSupport,
    scale: f64,
    limits: MaskLimits,
) -> Result<Option<BinaryMask>, MaskError> {
    match primitive {
        NeutralPrimitiveSupport::Fill {
            shape,
            local_to_surface,
        } => geometry_mask(
            &tessellate_fill(shape).map_err(|error| MaskError::Geometry(error.to_string()))?,
            *local_to_surface,
            scale,
            limits,
        ),
        NeutralPrimitiveSupport::Stroke {
            shape,
            style,
            local_to_surface,
        } => geometry_mask(
            &tessellate_stroke(shape, *style)
                .map_err(|error| MaskError::Geometry(error.to_string()))?,
            *local_to_surface,
            scale,
            limits,
        ),
        NeutralPrimitiveSupport::Image {
            destinations,
            local_to_surface,
        } => {
            let masks = destinations
                .iter()
                .map(|destination| {
                    let shape = SceneShape::rect(*destination);
                    let geometry = tessellate_fill(&shape)
                        .map_err(|error| MaskError::Geometry(error.to_string()))?;
                    geometry_mask(&geometry, *local_to_surface, scale, limits)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            union_masks(&masks, limits)
        }
        NeutralPrimitiveSupport::ShapedText { .. } => Err(MaskError::UnresolvedShapedText),
        NeutralPrimitiveSupport::ShapedTextPaths {
            paths,
            local_to_surface,
        } => {
            let masks = paths
                .iter()
                .map(|path| {
                    let shape = SceneShape::path(path.clone());
                    let geometry = tessellate_fill(&shape)
                        .map_err(|error| MaskError::Geometry(error.to_string()))?;
                    geometry_mask(&geometry, *local_to_surface, scale, limits)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            union_masks(&masks, limits)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct BinaryMask {
    origin_x: f64,
    origin_y: f64,
    width: u32,
    height: u32,
    samples: Vec<u8>,
}

impl BinaryMask {
    fn is_empty(&self) -> bool {
        self.samples.iter().all(|sample| *sample == 0)
    }

    fn into_alpha(self) -> AlphaMask {
        AlphaMask {
            origin_x: self.origin_x,
            origin_y: self.origin_y,
            width: self.width,
            height: self.height,
            alpha: self.samples.into(),
        }
    }

    fn sample(&self, surface_x: f64, surface_y: f64) -> bool {
        let local_x = (surface_x - self.origin_x).floor();
        let local_y = (surface_y - self.origin_y).floor();
        if !local_x.is_finite()
            || !local_y.is_finite()
            || local_x < 0.0
            || local_y < 0.0
            || local_x >= f64::from(self.width)
            || local_y >= f64::from(self.height)
        {
            return false;
        }
        let x = local_x as usize;
        let y = local_y as usize;
        self.samples[y * self.width as usize + x] != 0
    }
}

fn geometry_mask(
    geometry: &TessellatedGeometry,
    transform: LogicalTransform,
    scale: f64,
    limits: MaskLimits,
) -> Result<Option<BinaryMask>, MaskError> {
    if geometry.positions().is_empty() || geometry.indices().is_empty() {
        return Ok(None);
    }
    let transformed = geometry
        .positions()
        .iter()
        .copied()
        .map(|point| transform_physical(point, transform, scale))
        .collect::<Vec<_>>();
    let Some((origin_x, origin_y, width, height)) = point_workspace(&transformed, limits)? else {
        return Ok(None);
    };
    let mut mask = BinaryMask {
        origin_x,
        origin_y,
        width,
        height,
        samples: allocate_samples(width, height, limits)?,
    };
    for triangle in geometry.indices().chunks_exact(3) {
        let a = transformed[triangle[0] as usize];
        let b = transformed[triangle[1] as usize];
        let c = transformed[triangle[2] as usize];
        rasterize_triangle(&mut mask, a, b, c);
    }
    Ok((!mask.is_empty()).then_some(mask))
}

fn transform_physical(
    point: [f32; 2],
    transform: LogicalTransform,
    scale: f64,
) -> [f64; 2] {
    let [m11, m12, m21, m22, tx, ty] = transform.components().map(f64::from);
    let x = f64::from(point[0]);
    let y = f64::from(point[1]);
    [
        m11.mul_add(x, m21.mul_add(y, tx)) * scale,
        m12.mul_add(x, m22.mul_add(y, ty)) * scale,
    ]
}

fn point_workspace(
    points: &[[f64; 2]],
    limits: MaskLimits,
) -> Result<Option<(f64, f64, u32, u32)>, MaskError> {
    let Some(first) = points.first().copied() else {
        return Ok(None);
    };
    let mut min_x = first[0];
    let mut min_y = first[1];
    let mut max_x = first[0];
    let mut max_y = first[1];
    for [x, y] in points.iter().copied().skip(1) {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    workspace_from_bounds(min_x, min_y, max_x, max_y, limits)
}

fn workspace_from_bounds(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    limits: MaskLimits,
) -> Result<Option<(f64, f64, u32, u32)>, MaskError> {
    if ![min_x, min_y, max_x, max_y].into_iter().all(f64::is_finite) {
        return Err(MaskError::NonFiniteWorkspace);
    }
    let origin_x = min_x.floor();
    let origin_y = min_y.floor();
    let end_x = max_x.ceil();
    let end_y = max_y.ceil();
    if end_x <= origin_x || end_y <= origin_y {
        return Ok(None);
    }
    let width = end_x - origin_x;
    let height = end_y - origin_y;
    if width > f64::from(u32::MAX) || height > f64::from(u32::MAX) {
        return Err(MaskError::ExtentExceedsDeviceLimit {
            width: u64::MAX,
            height: u64::MAX,
            max_dimension: limits.max_dimension,
        });
    }
    let width = width as u32;
    let height = height as u32;
    validate_extent(width, height, limits)?;
    Ok(Some((origin_x, origin_y, width, height)))
}

fn validate_extent(width: u32, height: u32, limits: MaskLimits) -> Result<(), MaskError> {
    if width > limits.max_dimension || height > limits.max_dimension {
        return Err(MaskError::ExtentExceedsDeviceLimit {
            width: u64::from(width),
            height: u64::from(height),
            max_dimension: limits.max_dimension,
        });
    }
    let required = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(MaskError::AllocationOverflow)?;
    if required > limits.max_bytes {
        return Err(MaskError::AllocationExceedsDeviceLimit {
            required_bytes: required,
            max_bytes: limits.max_bytes,
        });
    }
    Ok(())
}

fn allocate_samples(width: u32, height: u32, limits: MaskLimits) -> Result<Vec<u8>, MaskError> {
    validate_extent(width, height, limits)?;
    let len = usize::try_from(u64::from(width) * u64::from(height))
        .map_err(|_| MaskError::AllocationOverflow)?;
    Ok(vec![0; len])
}

fn rasterize_triangle(mask: &mut BinaryMask, a: [f64; 2], b: [f64; 2], c: [f64; 2]) {
    let min_x = a[0].min(b[0]).min(c[0]).floor().max(mask.origin_x);
    let min_y = a[1].min(b[1]).min(c[1]).floor().max(mask.origin_y);
    let max_x = a[0]
        .max(b[0])
        .max(c[0])
        .ceil()
        .min(mask.origin_x + f64::from(mask.width));
    let max_y = a[1]
        .max(b[1])
        .max(c[1])
        .ceil()
        .min(mask.origin_y + f64::from(mask.height));
    let start_x = (min_x - mask.origin_x).max(0.0) as usize;
    let start_y = (min_y - mask.origin_y).max(0.0) as usize;
    let end_x = (max_x - mask.origin_x).max(0.0) as usize;
    let end_y = (max_y - mask.origin_y).max(0.0) as usize;
    let width = mask.width as usize;
    for y in start_y..end_y {
        let py = mask.origin_y + y as f64 + 0.5;
        for x in start_x..end_x {
            let px = mask.origin_x + x as f64 + 0.5;
            if point_in_triangle([px, py], a, b, c) {
                mask.samples[y * width + x] = 1;
            }
        }
    }
}

fn point_in_triangle(point: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    let ab = edge_sign(point, a, b);
    let bc = edge_sign(point, b, c);
    let ca = edge_sign(point, c, a);
    let has_negative = ab < 0.0 || bc < 0.0 || ca < 0.0;
    let has_positive = ab > 0.0 || bc > 0.0 || ca > 0.0;
    !(has_negative && has_positive)
}

fn edge_sign(point: [f64; 2], from: [f64; 2], to: [f64; 2]) -> f64 {
    (point[0] - to[0]) * (from[1] - to[1]) - (from[0] - to[0]) * (point[1] - to[1])
}

fn union_masks(masks: &[BinaryMask], limits: MaskLimits) -> Result<Option<BinaryMask>, MaskError> {
    let Some(first) = masks.first() else {
        return Ok(None);
    };
    let min_x = masks
        .iter()
        .map(|mask| mask.origin_x)
        .fold(first.origin_x, f64::min);
    let min_y = masks
        .iter()
        .map(|mask| mask.origin_y)
        .fold(first.origin_y, f64::min);
    let max_x = masks
        .iter()
        .map(|mask| mask.origin_x + f64::from(mask.width))
        .fold(first.origin_x + f64::from(first.width), f64::max);
    let max_y = masks
        .iter()
        .map(|mask| mask.origin_y + f64::from(mask.height))
        .fold(first.origin_y + f64::from(first.height), f64::max);
    let Some((origin_x, origin_y, width, height)) =
        workspace_from_bounds(min_x, min_y, max_x, max_y, limits)?
    else {
        return Ok(None);
    };
    let mut result = BinaryMask {
        origin_x,
        origin_y,
        width,
        height,
        samples: allocate_samples(width, height, limits)?,
    };
    let width_usize = width as usize;
    for y in 0..height as usize {
        let surface_y = origin_y + y as f64 + 0.5;
        for x in 0..width_usize {
            let surface_x = origin_x + x as f64 + 0.5;
            if masks.iter().any(|mask| mask.sample(surface_x, surface_y)) {
                result.samples[y * width_usize + x] = 1;
            }
        }
    }
    Ok((!result.is_empty()).then_some(result))
}

fn intersect_clip(mask: &mut BinaryMask, clip: &SceneClip, scale: f64) -> Result<(), MaskError> {
    let geometry = tessellate_fill(clip.shape()).map_err(|error| MaskError::Geometry(error.to_string()))?;
    if geometry.positions().is_empty() || geometry.indices().is_empty() {
        mask.samples.fill(0);
        return Ok(());
    }
    let transformed = geometry
        .positions()
        .iter()
        .copied()
        .map(|point| transform_physical(point, clip.clip_to_surface(), scale))
        .collect::<Vec<_>>();
    let width = mask.width as usize;
    for y in 0..mask.height as usize {
        let surface_y = mask.origin_y + y as f64 + 0.5;
        for x in 0..width {
            let index = y * width + x;
            if mask.samples[index] == 0 {
                continue;
            }
            let surface_x = mask.origin_x + x as f64 + 0.5;
            let covered = geometry.indices().chunks_exact(3).any(|triangle| {
                point_in_triangle(
                    [surface_x, surface_y],
                    transformed[triangle[0] as usize],
                    transformed[triangle[1] as usize],
                    transformed[triangle[2] as usize],
                )
            });
            if !covered {
                mask.samples[index] = 0;
            }
        }
    }
    Ok(())
}

fn signed_euclidean_spread(
    mask: BinaryMask,
    radius: f64,
    limits: MaskLimits,
) -> Result<BinaryMask, MaskError> {
    if radius == 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    if radius > 0.0 {
        dilate_euclidean(mask, radius, limits)
    } else {
        erode_euclidean(mask, -radius, limits)
    }
}

fn dilate_euclidean(
    mask: BinaryMask,
    radius: f64,
    limits: MaskLimits,
) -> Result<BinaryMask, MaskError> {
    let pad = radius.ceil() as u32;
    let padded = pad_mask(&mask, pad, limits)?;
    let distances = squared_distance_transform(&padded.samples, padded.width, padded.height, true);
    let threshold = radius * radius;
    let samples = distances
        .into_iter()
        .map(|distance| u8::from(distance <= threshold))
        .collect();
    Ok(BinaryMask { samples, ..padded })
}

fn erode_euclidean(
    mask: BinaryMask,
    radius: f64,
    limits: MaskLimits,
) -> Result<BinaryMask, MaskError> {
    let pad = radius.ceil().saturating_add(1.0) as u32;
    let padded = pad_mask(&mask, pad, limits)?;
    let distances = squared_distance_transform(&padded.samples, padded.width, padded.height, false);
    let threshold = (radius + EROSION_SAMPLE_SAFETY).powi(2);
    let padded_width = padded.width as usize;
    let source_width = mask.width as usize;
    let source_height = mask.height as usize;
    let pad_usize = pad as usize;
    let mut samples = vec![0; source_width.saturating_mul(source_height)];
    for y in 0..source_height {
        for x in 0..source_width {
            let source_index = y * source_width + x;
            if mask.samples[source_index] == 0 {
                continue;
            }
            let padded_index = (y + pad_usize) * padded_width + x + pad_usize;
            samples[source_index] = u8::from(distances[padded_index] > threshold);
        }
    }
    Ok(BinaryMask { samples, ..mask })
}

fn pad_mask(mask: &BinaryMask, pad: u32, limits: MaskLimits) -> Result<BinaryMask, MaskError> {
    if pad == 0 {
        return Ok(mask.clone());
    }
    let width = mask
        .width
        .checked_add(pad.saturating_mul(2))
        .ok_or(MaskError::AllocationOverflow)?;
    let height = mask
        .height
        .checked_add(pad.saturating_mul(2))
        .ok_or(MaskError::AllocationOverflow)?;
    let mut samples = allocate_samples(width, height, limits)?;
    let destination_width = width as usize;
    let source_width = mask.width as usize;
    let pad = pad as usize;
    for y in 0..mask.height as usize {
        let source = y * source_width;
        let destination = (y + pad) * destination_width + pad;
        samples[destination..destination + source_width]
            .copy_from_slice(&mask.samples[source..source + source_width]);
    }
    Ok(BinaryMask {
        origin_x: mask.origin_x - pad as f64,
        origin_y: mask.origin_y - pad as f64,
        width,
        height,
        samples,
    })
}

fn squared_distance_transform(samples: &[u8], width: u32, height: u32, feature: bool) -> Vec<f64> {
    let width = width as usize;
    let height = height as usize;
    let mut intermediate = vec![0.0; width.saturating_mul(height)];
    let mut column = vec![0.0; height];
    let mut transformed = vec![0.0; height];
    for x in 0..width {
        for y in 0..height {
            let is_feature = (samples[y * width + x] != 0) == feature;
            column[y] = if is_feature { 0.0 } else { DISTANCE_INFINITY };
        }
        edt_1d(&column, &mut transformed);
        for y in 0..height {
            intermediate[y * width + x] = transformed[y];
        }
    }
    let mut result = vec![0.0; width.saturating_mul(height)];
    let mut row = vec![0.0; width];
    let mut row_transformed = vec![0.0; width];
    for y in 0..height {
        let start = y * width;
        row.copy_from_slice(&intermediate[start..start + width]);
        edt_1d(&row, &mut row_transformed);
        result[start..start + width].copy_from_slice(&row_transformed);
    }
    result
}

fn edt_1d(input: &[f64], output: &mut [f64]) {
    if input.is_empty() {
        return;
    }
    let n = input.len();
    let mut locations = vec![0_usize; n];
    let mut boundaries = vec![0.0; n + 1];
    let mut envelope = 0_usize;
    locations[0] = 0;
    boundaries[0] = f64::NEG_INFINITY;
    boundaries[1] = f64::INFINITY;
    for q in 1..n {
        let mut intersection = parabola_intersection(input, q, locations[envelope]);
        while envelope > 0 && intersection <= boundaries[envelope] {
            envelope -= 1;
            intersection = parabola_intersection(input, q, locations[envelope]);
        }
        envelope += 1;
        locations[envelope] = q;
        boundaries[envelope] = intersection;
        boundaries[envelope + 1] = f64::INFINITY;
    }
    envelope = 0;
    for q in 0..n {
        while boundaries[envelope + 1] < q as f64 {
            envelope += 1;
        }
        let location = locations[envelope];
        let delta = q.abs_diff(location) as f64;
        output[q] = delta.mul_add(delta, input[location]);
    }
}

fn parabola_intersection(input: &[f64], left: usize, right: usize) -> f64 {
    let left = left as f64;
    let right = right as f64;
    let numerator = (input[left as usize] + left * left) - (input[right as usize] + right * right);
    numerator / (2.0 * (left - right))
}

fn square_dilate(
    mask: BinaryMask,
    radius: f64,
    limits: MaskLimits,
) -> Result<BinaryMask, MaskError> {
    let radius = radius.floor().max(0.0) as u32;
    if radius == 0 || mask.is_empty() {
        return Ok(mask);
    }
    let padded = pad_mask(&mask, radius, limits)?;
    let width = padded.width as usize;
    let height = padded.height as usize;
    let radius = radius as usize;
    let mut horizontal = vec![0_u8; padded.samples.len()];
    for y in 0..height {
        let mut prefix = vec![0_u32; width + 1];
        for x in 0..width {
            prefix[x + 1] = prefix[x] + u32::from(padded.samples[y * width + x] != 0);
        }
        for x in 0..width {
            let start = x.saturating_sub(radius);
            let end = x.saturating_add(radius).saturating_add(1).min(width);
            horizontal[y * width + x] = u8::from(prefix[end] != prefix[start]);
        }
    }
    let mut samples = vec![0_u8; padded.samples.len()];
    for x in 0..width {
        let mut prefix = vec![0_u32; height + 1];
        for y in 0..height {
            prefix[y + 1] = prefix[y] + u32::from(horizontal[y * width + x] != 0);
        }
        for y in 0..height {
            let start = y.saturating_sub(radius);
            let end = y.saturating_add(radius).saturating_add(1).min(height);
            samples[y * width + x] = u8::from(prefix[end] != prefix[start]);
        }
    }
    Ok(BinaryMask { samples, ..padded })
}

fn gaussian_blur(
    mask: BinaryMask,
    sigma: f64,
    blur_radius: f64,
    limits: MaskLimits,
) -> Result<BinaryMask, MaskError> {
    let radius = blur_radius.floor().max(0.0) as u32;
    if radius == 0 || sigma <= 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    let padded = pad_mask(&mask, radius, limits)?;
    let radius = radius as usize;
    let weights = (0..=radius)
        .map(|offset| (-0.5 * (offset as f64 / sigma).powi(2)).exp())
        .collect::<Vec<_>>();
    let normalization = weights[0] + 2.0 * weights.iter().skip(1).sum::<f64>();
    let weights = weights
        .into_iter()
        .map(|weight| weight / normalization)
        .collect::<Vec<_>>();
    let width = padded.width as usize;
    let height = padded.height as usize;
    let mut horizontal = vec![0.0_f64; padded.samples.len()];
    for y in 0..height {
        for x in 0..width {
            let mut value = weights[0] * f64::from(padded.samples[y * width + x]);
            for offset in 1..=radius {
                let weight = weights[offset];
                if let Some(left) = x.checked_sub(offset) {
                    value += weight * f64::from(padded.samples[y * width + left]);
                }
                let right = x + offset;
                if right < width {
                    value += weight * f64::from(padded.samples[y * width + right]);
                }
            }
            horizontal[y * width + x] = value;
        }
    }
    let mut samples = vec![0_u8; padded.samples.len()];
    for y in 0..height {
        for x in 0..width {
            let mut value = weights[0] * horizontal[y * width + x];
            for offset in 1..=radius {
                let weight = weights[offset];
                if let Some(top) = y.checked_sub(offset) {
                    value += weight * horizontal[top * width + x];
                }
                let bottom = y + offset;
                if bottom < height {
                    value += weight * horizontal[bottom * width + x];
                }
            }
            samples[y * width + x] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    Ok(BinaryMask { samples, ..padded })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use runenui_core::{LogicalRect, LogicalTransform, SceneShape};
    use runenui_runtime::RasterScale;

    use super::{MaskLimits, prepare_visual_shadow};
    use crate::backend::clipped::resource::group::support::{
        NeutralPrimitiveSupport, NeutralShadowFacts, NeutralSupport,
    };

    fn rect_support() -> Arc<NeutralSupport> {
        Arc::new(NeutralSupport::Primitive(NeutralPrimitiveSupport::Fill {
            shape: SceneShape::rect(
                LogicalRect::try_new(0.0, 0.0, 8.0, 8.0)
                    .unwrap_or_else(|_| unreachable!("controlled rectangle is valid")),
            ),
            local_to_surface: LogicalTransform::IDENTITY,
        }))
    }

    fn limits() -> MaskLimits {
        MaskLimits::new(256, 256 * 256)
    }

    #[test]
    fn positive_and_negative_spread_use_euclidean_distance() {
        let scale = RasterScale::ONE;
        let expanded = prepare_visual_shadow(
            &rect_support(),
            NeutralShadowFacts::new(0.0, 0.0, 0.0, 2.0),
            scale,
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled mask resolves"))
        .unwrap_or_else(|| unreachable!("positive spread remains visible"));
        assert!(expanded.width() >= 12 && expanded.height() >= 12);
        let eroded = prepare_visual_shadow(
            &rect_support(),
            NeutralShadowFacts::new(0.0, 0.0, 0.0, -2.0),
            scale,
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled erosion resolves"))
        .unwrap_or_else(|| unreachable!("8px square survives 2px erosion"));
        assert!(eroded.alpha().iter().filter(|sample| **sample != 0).count() < 64);
    }

    #[test]
    fn complete_erosion_produces_no_visual_shadow() {
        let result = prepare_visual_shadow(
            &rect_support(),
            NeutralShadowFacts::new(0.0, 0.0, 0.0, -8.0),
            RasterScale::ONE,
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled erosion resolves"));
        assert!(result.is_none());
    }

    #[test]
    fn offset_is_preserved_as_fractional_workspace_origin() {
        let shadow = prepare_visual_shadow(
            &rect_support(),
            NeutralShadowFacts::new(1.25, -2.5, 0.0, 0.0),
            RasterScale::ONE,
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled offset resolves"))
        .unwrap_or_else(|| unreachable!("offset shadow remains visible"));
        assert!((shadow.origin_x() - 1.25).abs() < f64::EPSILON);
        assert!((shadow.origin_y() + 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn gaussian_output_is_zero_beyond_truncated_three_sigma_workspace() {
        let shadow = prepare_visual_shadow(
            &rect_support(),
            NeutralShadowFacts::new(0.0, 0.0, 1.0, 0.0),
            RasterScale::ONE,
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled blur resolves"))
        .unwrap_or_else(|| unreachable!("blurred shadow remains visible"));
        assert_eq!(shadow.width(), 14);
        assert_eq!(shadow.height(), 14);
    }
}
