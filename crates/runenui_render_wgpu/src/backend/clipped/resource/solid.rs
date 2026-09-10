//! Disposable real-wgpu realization for one solid fill/stroke paint item.
//!
//! Lyon output is consumed only as a private coverage decomposition. Triangle
//! overlap writes one idempotent stencil coverage bit; the logical item's color
//! and opacity are then source-over composited exactly once per covered sample.

use std::collections::HashMap;

use runenui_core::{Color, LogicalTransform, SceneOpacity, SceneShape, StrokeStyle};
use runenui_runtime::{RasterScale, SceneClip};
use wgpu::util::DeviceExt;

use crate::tessellation::{
    TessellatedGeometry, TessellationError, tessellate_fill, tessellate_stroke,
};

use super::super::super::{
    OffscreenExtent, OffscreenRenderError, RasterCanvasExtent, clip_polygon_to_canvas,
    physical_point_to_ndc, srgb8_to_linear_f32,
};
use super::super::{
    ClipTargetPipelines, STENCIL_ALLOWED, STENCIL_FORMAT, apply_clip_mask, prepare_clip_uniforms,
};

const COVERAGE_VERTEX_SIZE: usize = 8;
const COVERAGE_VERTEX_STRIDE: u64 = 8;
const COVERAGE_SHADER: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) @invariant position: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    return output;
}

@fragment
fn fs_main() {}
";

const SHADE_SHADER: &str = r"
struct ShadeUniform {
    color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> shade: ShadeUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return shade.color;
}
";

const COVERAGE_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x2,
    offset: 0,
    shader_location: 0,
}];

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SupportedSolid {
    geometry: TessellatedGeometry,
    color: Color,
    opacity: SceneOpacity,
    local_to_surface: LogicalTransform,
    clips: Vec<SceneClip>,
}

impl SupportedSolid {
    pub(super) fn fill(
        shape: &SceneShape,
        color: Color,
        opacity: SceneOpacity,
        local_to_surface: LogicalTransform,
        clips: Vec<SceneClip>,
    ) -> Result<Self, TessellationError> {
        tessellate_fill(shape).map(|geometry| Self {
            geometry,
            color,
            opacity,
            local_to_surface,
            clips,
        })
    }

    pub(super) fn stroke(
        shape: &SceneShape,
        style: StrokeStyle,
        color: Color,
        opacity: SceneOpacity,
        local_to_surface: LogicalTransform,
        clips: Vec<SceneClip>,
    ) -> Result<Self, TessellationError> {
        tessellate_stroke(shape, style).map(|geometry| Self {
            geometry,
            color,
            opacity,
            local_to_surface,
            clips,
        })
    }

    pub(super) const fn has_clips(&self) -> bool {
        !self.clips.is_empty()
    }
}

#[derive(Debug)]
struct SolidTargetPipelines {
    coverage: wgpu::RenderPipeline,
    shade: wgpu::RenderPipeline,
    shade_bind_group_layout: wgpu::BindGroupLayout,
}

#[derive(Debug, Default)]
pub(super) struct SolidRenderer {
    pipelines: HashMap<wgpu::TextureFormat, SolidTargetPipelines>,
}

impl SolidRenderer {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn ensure_pipelines(
        &mut self,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> Result<(), OffscreenRenderError> {
        if !matches!(
            target_format,
            wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(OffscreenRenderError::UnsupportedTargetFormat {
                format: target_format,
            });
        }
        self.pipelines.entry(target_format).or_insert_with(|| {
            let shade_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("runenui solid shade bind-group layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
            SolidTargetPipelines {
                coverage: create_coverage_pipeline(device),
                shade: create_shade_pipeline(device, target_format, &shade_bind_group_layout),
                shade_bind_group_layout,
            }
        });
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the solid realization boundary keeps the exact target/canvas/scale, optional accepted clip machinery, and logical item input explicit"
    )]
    pub(super) fn encode_item(
        &self,
        device: &wgpu::Device,
        clip_pipelines: Option<&ClipTargetPipelines>,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        extent: OffscreenExtent,
        canvas_extent: RasterCanvasExtent,
        raster_scale: RasterScale,
        item: &SupportedSolid,
    ) {
        let vertex_bytes = coverage_vertex_bytes(item, extent, canvas_extent, raster_scale);
        if vertex_bytes.is_empty() {
            return;
        }
        let vertex_count =
            u32::try_from(vertex_bytes.len() / COVERAGE_VERTEX_SIZE).unwrap_or(u32::MAX);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("runenui solid primitive coverage vertices"),
            contents: &vertex_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let pipelines = self
            .pipelines
            .get(&target_format)
            .unwrap_or_else(|| unreachable!("solid target pipelines were ensured"));

        clear_coverage(encoder, stencil_view);
        draw_coverage(
            encoder,
            stencil_view,
            &pipelines.coverage,
            &vertex_buffer,
            vertex_count,
        );

        if item.has_clips() {
            let Some(clip_uniforms) = prepare_clip_uniforms(&item.clips, raster_scale) else {
                return;
            };
            let clip_pipelines = clip_pipelines
                .unwrap_or_else(|| unreachable!("solid item clips require clip pipelines"));
            for uniform in &clip_uniforms {
                apply_clip_mask(device, encoder, stencil_view, &clip_pipelines.mask, uniform);
            }
        }

        shade_once(
            device,
            encoder,
            color_view,
            stencil_view,
            pipelines,
            item.color,
            item.opacity,
        );
    }
}

fn create_coverage_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui solid primitive coverage shader"),
        source: wgpu::ShaderSource::Wgsl(COVERAGE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui solid primitive coverage pipeline layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("runenui solid primitive coverage pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: COVERAGE_VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &COVERAGE_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(coverage_stencil_state()),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_shade_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui solid primitive shade shader"),
        source: wgpu::ShaderSource::Wgsl(SHADE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui solid primitive shade pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("runenui solid primitive shade pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(shade_stencil_state()),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

const fn coverage_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Always,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Replace,
    }
}

fn coverage_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: coverage_stencil_face(),
            back: coverage_stencil_face(),
            read_mask: 0xff,
            write_mask: 0xff,
        },
    )
}

const fn shade_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Keep,
    }
}

fn shade_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: shade_stencil_face(),
            back: shade_stencil_face(),
            read_mask: 0xff,
            write_mask: 0,
        },
    )
}

fn clear_coverage(encoder: &mut wgpu::CommandEncoder, stencil_view: &wgpu::TextureView) {
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(0),
            store: wgpu::StoreOp::Store,
        }),
    };
    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui solid primitive coverage reset pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

fn draw_coverage(
    encoder: &mut wgpu::CommandEncoder,
    stencil_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    vertex_count: u32,
) {
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        }),
    };
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui solid primitive coverage pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(pipeline);
    render_pass.set_stencil_reference(STENCIL_ALLOWED);
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    render_pass.draw(0..vertex_count, 0..1);
}

fn shade_once(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    stencil_view: &wgpu::TextureView,
    pipelines: &SolidTargetPipelines,
    color: Color,
    opacity: SceneOpacity,
) {
    let shade = shade_bytes(color, opacity);
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("runenui solid primitive shade uniform"),
        contents: &shade,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("runenui solid primitive shade bind group"),
        layout: &pipelines.shade_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: color_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        },
    });
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        }),
    };
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui solid primitive one-source shade pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(&pipelines.shade);
    render_pass.set_stencil_reference(STENCIL_ALLOWED);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.draw(0..3, 0..1);
}

fn shade_bytes(color: Color, opacity: SceneOpacity) -> [u8; 16] {
    let values = [
        srgb8_to_linear_f32(color.red()),
        srgb8_to_linear_f32(color.green()),
        srgb8_to_linear_f32(color.blue()),
        f32::from(color.alpha()) / 255.0 * opacity.get(),
    ];
    let mut bytes = [0_u8; 16];
    for (destination, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(values) {
        destination.copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

fn coverage_vertex_bytes(
    item: &SupportedSolid,
    extent: OffscreenExtent,
    canvas_extent: RasterCanvasExtent,
    raster_scale: RasterScale,
) -> Vec<u8> {
    if item.local_to_surface.inverse().is_none() {
        return Vec::new();
    }
    let [m11, m12, m21, m22, tx, ty] = item.local_to_surface.components().map(f64::from);
    let scale = f64::from(raster_scale.get());
    let positions = item.geometry.positions();
    let mut bytes = Vec::with_capacity(item.geometry.indices().len().saturating_mul(8));

    for triangle in item.geometry.indices().chunks_exact(3) {
        let mut polygon = Vec::with_capacity(3);
        for index in triangle {
            let index = usize::try_from(*index)
                .unwrap_or_else(|_| unreachable!("validated tessellation index fits usize"));
            let [x, y] = positions[index].map(f64::from);
            polygon.push([
                m11.mul_add(x, m21.mul_add(y, tx)) * scale,
                m12.mul_add(x, m22.mul_add(y, ty)) * scale,
            ]);
        }
        let polygon = clip_polygon_to_canvas(polygon, canvas_extent);
        if polygon.len() < 3 {
            continue;
        }
        let positions = polygon
            .into_iter()
            .map(|point| physical_point_to_ndc(point, extent))
            .collect::<Vec<_>>();
        for index in 1..positions.len() - 1 {
            for position in [positions[0], positions[index], positions[index + 1]] {
                for component in position {
                    bytes.extend_from_slice(&component.to_ne_bytes());
                }
            }
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use runenui_core::{Color, LogicalLength, LogicalRect, SceneShape, StrokeStyle};

    use super::shade_bytes;

    #[test]
    fn shade_uniform_multiplies_item_opacity_into_alpha_once() {
        let bytes = shade_bytes(
            Color::rgba(0x80, 0x40, 0x20, 0x80),
            runenui_core::SceneOpacity::new(0.5)
                .unwrap_or_else(|_| unreachable!("test opacity is valid")),
        );
        let alpha = f32::from_ne_bytes(
            bytes[12..16]
                .try_into()
                .unwrap_or_else(|_| unreachable!("shade alpha occupies four bytes")),
        );
        assert!((alpha - (128.0 / 255.0 * 0.5)).abs() < f32::EPSILON);
    }

    #[test]
    fn zero_width_stroke_prepares_empty_geometry() {
        let rect = LogicalRect::try_new(0.0, 0.0, 10.0, 10.0)
            .unwrap_or_else(|_| unreachable!("test rect is valid"));
        let item = super::SupportedSolid::stroke(
            &SceneShape::rect(rect),
            StrokeStyle::new(LogicalLength::ZERO),
            Color::BLACK,
            runenui_core::SceneOpacity::OPAQUE,
            runenui_core::LogicalTransform::IDENTITY,
            Vec::new(),
        )
        .unwrap_or_else(|_| unreachable!("zero stroke realization is valid"));
        assert!(item.geometry.positions().is_empty());
    }
}
