mod image;
#[allow(
    clippy::chunks_exact_to_as_chunks,
    clippy::missing_errors_doc,
    clippy::too_many_lines,
    reason = "the private mixed-scene implementation is wrapped by the documented public facade; its long atomic preflight and explicit triangle chunk iteration remain implementation details"
)]
mod resource;
mod shaped;

pub use resource::{PublicationRenderError, UnsupportedShapedGlyphKind};

use std::collections::HashMap;

use runenui_core::{Color, SceneShape};
use runenui_runtime::{PaintPublication, RasterScale, SceneClip};
use wgpu::util::DeviceExt;

use crate::{ResourceProvider, WgpuHasDisplayHandle, observation::PublicationObservation};

const STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Stencil8;
const STENCIL_ALLOWED: u32 = 1;
const CLIP_MASK_SHADER: &str = r"
struct ClipUniform {
    transform_a: vec4<f32>,
    transform_b: vec4<f32>,
    rect: vec4<f32>,
    radii: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> clip: ClipUniform;

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

fn outside_circle(point: vec2<f32>, center: vec2<f32>, radius: f32) -> bool {
    if radius <= 0.0 {
        return false;
    }
    let delta = point - center;
    return dot(delta, delta) > radius * radius;
}

fn contains_clip(local: vec2<f32>) -> bool {
    let left = clip.rect.x;
    let top = clip.rect.y;
    let right = clip.rect.z;
    let bottom = clip.rect.w;
    if !(local.x >= left && local.x < right && local.y >= top && local.y < bottom) {
        return false;
    }
    if clip.transform_b.w < 0.5 {
        return true;
    }

    let top_left = clip.radii.x;
    if local.x < left + top_left && local.y < top + top_left
        && outside_circle(local, vec2<f32>(left + top_left, top + top_left), top_left)
    {
        return false;
    }

    let top_right = clip.radii.y;
    if local.x >= right - top_right && local.y < top + top_right
        && outside_circle(local, vec2<f32>(right - top_right, top + top_right), top_right)
    {
        return false;
    }

    let bottom_right = clip.radii.z;
    if local.x >= right - bottom_right && local.y >= bottom - bottom_right
        && outside_circle(
            local,
            vec2<f32>(right - bottom_right, bottom - bottom_right),
            bottom_right,
        )
    {
        return false;
    }

    let bottom_left = clip.radii.w;
    if local.x < left + bottom_left && local.y >= bottom - bottom_left
        && outside_circle(
            local,
            vec2<f32>(left + bottom_left, bottom - bottom_left),
            bottom_left,
        )
    {
        return false;
    }

    return true;
}

@fragment
fn fs_main(input: VertexOutput) {
    let surface = input.position.xy / clip.transform_b.z;
    let local = vec2<f32>(
        clip.transform_a.x * surface.x + clip.transform_a.z * surface.y + clip.transform_b.x,
        clip.transform_a.y * surface.x + clip.transform_a.w * surface.y + clip.transform_b.y,
    );
    if contains_clip(local) {
        discard;
    }
}
";

#[derive(Debug)]
struct ClipTargetPipelines {
    mask: ClipMaskPipeline,
}

#[derive(Debug)]
struct ClipMaskPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ClipMaskPipeline {
    fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("runenui clip-mask bind-group layout"),
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("runenui clip-mask pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("runenui clip-mask shader"),
            source: wgpu::ShaderSource::Wgsl(CLIP_MASK_SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("runenui clip-mask pipeline"),
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
            depth_stencil: Some(mask_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

/// Canonical production renderer over retained `RunenUI` paint publications.
///
/// The renderer owns only disposable GPU realization, target state, resource
/// caches, and presentation lineage. Runtime publication remains the semantic
/// authority; unsupported renderer breadth fails closed during preflight.
#[derive(Debug)]
pub struct ResourceRenderer {
    inner: resource::ResourceRenderer,
}

impl ResourceRenderer {
    /// Selects a native adapter and creates a renderer-owned wgpu device and queue.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request(options: super::RendererOptions) -> Result<Self, super::RendererInitError> {
        resource::ResourceRenderer::request(options)
            .await
            .map(|inner| Self { inner })
    }

    /// Selects a native adapter using a caller-owned display connection.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request_with_display_handle(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, super::RendererInitError> {
        resource::ResourceRenderer::request_with_display_handle(options, display)
            .await
            .map(|inner| Self { inner })
    }

    /// Creates and retains a native surface before selecting a compatible adapter.
    ///
    /// # Errors
    ///
    /// Returns structured surface-creation, compatible-adapter, target-format, or
    /// device diagnostics when construction fails.
    pub async fn request_with_surface_target(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
        window: impl wgpu::WindowHandle + 'static,
    ) -> Result<Self, super::RendererInitError> {
        resource::ResourceRenderer::request_with_surface_target(options, display, window)
            .await
            .map(|inner| Self { inner })
    }

    /// Returns immutable instance, adapter, device, and target diagnostics.
    #[must_use]
    pub const fn diagnostics(&self) -> &super::RendererDiagnostics {
        self.inner.diagnostics()
    }

    /// Returns the immutable observation for the most recent publication attempt.
    #[must_use]
    pub const fn last_observation(&self) -> Option<&crate::PublicationObservation> {
        self.inner.last_observation()
    }

    /// Returns whether construction retained an actual native surface target.
    #[must_use]
    pub const fn has_surface(&self) -> bool {
        self.inner.has_surface()
    }

    /// Returns the exact configured native surface extent, when configured.
    #[must_use]
    pub const fn configured_surface_extent(&self) -> Option<super::OffscreenExtent> {
        self.inner.configured_surface_extent()
    }

    /// Returns the renderer-local generation of the current native surface configuration.
    #[must_use]
    pub const fn surface_target_generation(&self) -> u64 {
        self.inner.surface_target_generation()
    }

    /// Configures the retained native surface for one non-zero physical extent.
    ///
    /// Reconfiguration creates a new renderer-local target generation and forgets
    /// successful surface-publication lineage. Resource uploads remain disposable
    /// renderer state and may be reused across target recreation.
    ///
    /// # Errors
    ///
    /// Returns a structured error when no native surface exists, the extent is
    /// invalid for the selected device, or the renderer cannot allocate another
    /// target generation.
    pub fn configure_surface(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<super::OffscreenExtent, PublicationRenderError> {
        self.inner.configure_surface(width, height)
    }

    /// Drops the retained offscreen target and every publication realization tied to it.
    #[must_use]
    pub fn discard_offscreen_target(&mut self) -> bool {
        self.inner.discard_offscreen_target()
    }

    /// Drops renderer-owned uploaded resource realizations without changing logical refs.
    ///
    /// A real cache loss also invalidates successful publication lineage so the
    /// next complete publication is reconstructed with a full resync on every target.
    #[must_use]
    pub fn discard_resource_cache(&mut self) -> bool {
        self.inner.discard_resource_cache()
    }

    /// Renders one complete publication and reads actual GPU bytes.
    ///
    /// Generic solid fill/stroke geometry, images, and shaped text share one
    /// ordered target transaction. Scene validation, solid tessellation, and
    /// resource preflight complete before retained-target mutation.
    ///
    /// # Errors
    ///
    /// Returns deterministic scene, solid-realization, resource, target, device,
    /// or readback failures. Preflight failures do not mutate the retained target.
    pub fn render_offscreen_publication<P: ResourceProvider + ?Sized>(
        &mut self,
        publication: &PaintPublication,
        provider: &P,
    ) -> Result<super::OffscreenPublicationReadback, PublicationRenderError> {
        self.inner.render_offscreen_publication(publication, provider)
    }

    /// Renders one complete publication directly into the configured native surface.
    ///
    /// The configured surface extent is the physical target authority. A newly
    /// acquired swapchain image is always completely rendered; publication lineage
    /// does not imply that the acquired image already contains current pixels.
    /// `before_present` is invoked exactly once after successful submission and
    /// immediately before presentation.
    ///
    /// # Errors
    ///
    /// Returns deterministic publication/resource/backend failures plus structured
    /// native-surface recovery states. Timeout and occlusion may be retried later;
    /// outdated/suboptimal targets should be reconfigured; a lost surface requires
    /// recreating the renderer. `before_present` is not invoked before successful
    /// GPU submission.
    pub fn render_surface_publication<P: ResourceProvider + ?Sized>(
        &mut self,
        publication: &PaintPublication,
        provider: &P,
        before_present: impl FnOnce(),
    ) -> Result<crate::PublicationObservation, PublicationRenderError> {
        self.inner
            .render_surface_publication(publication, provider, before_present)
    }

    /// Executes one real wgpu clear and returns actual texture bytes from GPU readback.
    ///
    /// This low-level diagnostic consumes no publication and never changes
    /// publication lineage.
    ///
    /// # Errors
    ///
    /// Returns structured extent, device-wait, buffer-map, or mapped-range failures.
    pub fn clear_offscreen(
        &self,
        extent: super::OffscreenExtent,
        color: Color,
    ) -> Result<super::OffscreenReadback, super::OffscreenRenderError> {
        self.inner.clear_offscreen(extent, color)
    }
}

/// Renderer-local clip infrastructure retained underneath the mixed public facade.
#[derive(Debug)]
struct Renderer {
    base: super::Renderer,
    clip_pipelines: HashMap<wgpu::TextureFormat, ClipTargetPipelines>,
}

impl Renderer {
    async fn request(
        options: super::RendererOptions,
    ) -> Result<Self, super::RendererInitError> {
        super::Renderer::request(options).await.map(Self::from_base)
    }

    async fn request_with_display_handle(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, super::RendererInitError> {
        super::Renderer::request_with_display_handle(options, display)
            .await
            .map(Self::from_base)
    }

    async fn request_with_surface_target(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
        window: impl wgpu::WindowHandle + 'static,
    ) -> Result<Self, super::RendererInitError> {
        super::Renderer::request_with_surface_target(options, display, window)
            .await
            .map(Self::from_base)
    }

    fn from_base(base: super::Renderer) -> Self {
        Self {
            base,
            clip_pipelines: HashMap::new(),
        }
    }

    const fn diagnostics(&self) -> &super::RendererDiagnostics {
        self.base.diagnostics()
    }

    const fn last_observation(&self) -> Option<&PublicationObservation> {
        self.base.last_observation()
    }

    const fn has_surface(&self) -> bool {
        self.base.has_surface()
    }

    fn discard_offscreen_target(&mut self) -> bool {
        self.base.discard_offscreen_target()
    }

    fn clear_offscreen(
        &self,
        extent: super::OffscreenExtent,
        color: Color,
    ) -> Result<super::OffscreenReadback, super::OffscreenRenderError> {
        self.base.clear_offscreen(extent, color)
    }

    fn ensure_clip_pipelines(
        &mut self,
        format: wgpu::TextureFormat,
    ) -> Result<(), super::OffscreenRenderError> {
        if !matches!(
            format,
            wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(super::OffscreenRenderError::UnsupportedTargetFormat { format });
        }
        let device = &self.base.device;
        self.clip_pipelines
            .entry(format)
            .or_insert_with(|| ClipTargetPipelines {
                mask: ClipMaskPipeline::new(device),
            });
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ClipUniform {
    values: [f32; 16],
}

impl ClipUniform {
    fn from_scene_clip(clip: &SceneClip, raster_scale: RasterScale) -> Option<Self> {
        let surface_to_clip = clip.clip_to_surface().inverse()?;
        let [m11, m12, m21, m22, tx, ty] = surface_to_clip.components();
        let (rect, radii, shape_kind) = match clip.shape() {
            SceneShape::Rect(rect) => (*rect, [0.0; 4], 0.0),
            SceneShape::RoundedRect { rect, .. } => {
                (*rect, normalized_clip_radii(clip.shape())?, 1.0)
            }
            SceneShape::Ellipse(_) | SceneShape::Path(_) => return None,
        };
        Some(Self {
            values: [
                m11,
                m12,
                m21,
                m22,
                tx,
                ty,
                raster_scale.get(),
                shape_kind,
                rect.x(),
                rect.y(),
                rect.max_x(),
                rect.max_y(),
                radii[0],
                radii[1],
                radii[2],
                radii[3],
            ],
        })
    }

    fn bytes(&self) -> [u8; 64] {
        let mut bytes = [0_u8; 64];
        for (destination, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(self.values) {
            destination.copy_from_slice(&value.to_ne_bytes());
        }
        bytes
    }
}

fn prepare_clip_uniforms(
    clips: &[SceneClip],
    raster_scale: RasterScale,
) -> Option<Vec<ClipUniform>> {
    clips
        .iter()
        .map(|clip| ClipUniform::from_scene_clip(clip, raster_scale))
        .collect()
}

fn normalized_clip_radii(shape: &SceneShape) -> Option<[f32; 4]> {
    let radius = shape.normalized_radius()?;
    Some([
        radius.top_left().get(),
        radius.top_right().get(),
        radius.bottom_right().get(),
        radius.bottom_left().get(),
    ])
}

const fn mask_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Always,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Zero,
    }
}

fn mask_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: mask_stencil_face(),
            back: mask_stencil_face(),
            read_mask: 0xff,
            write_mask: 0xff,
        },
    )
}

const fn clipped_fill_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Keep,
    }
}

fn clipped_fill_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: clipped_fill_stencil_face(),
            back: clipped_fill_stencil_face(),
            read_mask: 0xff,
            write_mask: 0,
        },
    )
}

fn create_stencil_target(
    device: &wgpu::Device,
    extent: super::OffscreenExtent,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("runenui scene stencil target"),
        size: super::texture_extent(extent),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: STENCIL_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn clear_color_target(encoder: &mut wgpu::CommandEncoder, color_view: &wgpu::TextureView) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: color_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
        },
    });
    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui scene clear pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

fn clear_stencil_mask(encoder: &mut wgpu::CommandEncoder, stencil_view: &wgpu::TextureView) {
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(STENCIL_ALLOWED),
            store: wgpu::StoreOp::Store,
        }),
    };
    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui clip stencil reset pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

fn apply_clip_mask(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    stencil_view: &wgpu::TextureView,
    pipeline: &ClipMaskPipeline,
    uniform: &ClipUniform,
) {
    let uniform_bytes = uniform.bytes();
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("runenui clip-mask uniform"),
        contents: &uniform_bytes,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("runenui clip-mask bind group"),
        layout: &pipeline.bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
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
        label: Some("runenui conjunctive clip-mask pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(&pipeline.pipeline);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.draw(0..3, 0..1);
}
