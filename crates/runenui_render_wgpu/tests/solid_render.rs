#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{
    sync::Arc,
    task::{Context, Wake, Waker},
    thread,
};

use runenui_core::{
    Brush, Color, ContributionClip, Element, GradientStop, GradientStops, LinearGradient,
    LogicalLength, LogicalPoint, LogicalRect, LogicalSize, LogicalTransform, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionItem, PathFillRule, PathVerb,
    RadialGradient, ResourceRef, ScenePath, SceneShape, StrokeCap, StrokeJoin, StrokeStyle,
    StyleEnvironment, UiApp, UnitInterval, Widget, WidgetMeasure,
};
use runenui_render_wgpu::{
    BackendSelection, PublicationUpdateMode, Renderer, RendererInitError, RendererOptions,
    ResourcePayload, ResourceProvider, ResourceProviderError, ResourceProviderErrorKind,
    ResourceRequest,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintPublication, RasterScale, SurfaceBuildContext,
};

const SURFACE_WIDTH: u16 = 64;
const SURFACE_HEIGHT: u16 = 48;

#[derive(Clone, Debug)]
struct SceneFixture {
    items: Vec<PaintContributionItem>,
}

impl Widget<Vec<PaintContributionItem>> for SceneFixture {
    type State = Vec<PaintContributionItem>;

    fn create_state(&self) -> Self::State {
        self.items.clone()
    }

    fn measure(&self, _: &Self::State, _: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::from(SURFACE_WIDTH),
            LogicalLength::from(SURFACE_HEIGHT),
        )
    }

    fn paint(&self, items: &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(items.clone())
    }
}

struct FixtureApp;

impl UiApp for FixtureApp {
    type State = Vec<PaintContributionItem>;
    type Action = Vec<PaintContributionItem>;
    type HostProtocol = NoHostProtocol;

    fn root(items: &Self::State) -> Element<Self::Action> {
        Element::new(SceneFixture {
            items: items.clone(),
        })
    }

    fn update(items: &mut Self::State, replacement: Self::Action) {
        *items = replacement;
    }
}

struct NoResources;

impl ResourceProvider for NoResources {
    fn load(
        &self,
        _: &ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Malformed,
            "solid-render proof unexpectedly requested a resource",
        ))
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("fixture point is finite"))
}

fn path(verbs: Vec<PathVerb>) -> SceneShape {
    SceneShape::path(
        ScenePath::new(verbs, PathFillRule::NonZero)
            .unwrap_or_else(|_| unreachable!("fixture path is structurally valid")),
    )
}

fn gradient_stops(entries: &[(f32, Color)]) -> GradientStops {
    GradientStops::new(
        entries
            .iter()
            .map(|(offset, color)| {
                GradientStop::new(
                    UnitInterval::new(*offset)
                        .unwrap_or_else(|_| unreachable!("fixture stop offset is valid")),
                    *color,
                )
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| unreachable!("fixture gradient stops are valid"))
}

fn publication(items: Vec<PaintContributionItem>) -> PaintPublication {
    let mut runtime = AppRuntime::<FixtureApp>::mount(items);
    let environment = StyleEnvironment::default();
    let logical_size = LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
        .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::tight(logical_size))
        .with_raster_scale(
            RasterScale::new(1.0).unwrap_or_else(|_| unreachable!("fixture raster scale is valid")),
        );
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
        .paint_publication()
        .clone()
}

#[test]
fn real_gpu_generic_solids_preserve_shape_transform_clip_and_degenerate_semantics()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let green = Color::rgb(0x35, 0xB8, 0x68);
    let red = Color::rgb(0xD9, 0x4E, 0x49);
    let blue = Color::rgb(0x45, 0x79, 0xD8);

    let triangle = path(vec![
        PathVerb::MoveTo(point(4.0, 4.0)),
        PathVerb::LineTo(point(28.0, 4.0)),
        PathVerb::LineTo(point(16.0, 28.0)),
        PathVerb::Close,
    ]);
    let clipped_triangle = PaintContributionItem::fill(triangle, Brush::solid(green)).with_clip(
        ContributionClip::identity(SceneShape::rect(rect(10.0, 0.0, 12.0, 32.0))),
    );
    let transformed_ellipse = PaintContributionItem::fill(
        SceneShape::ellipse(rect(0.0, 0.0, 12.0, 10.0)),
        Brush::solid(red),
    )
    .with_transform(LogicalTransform::translation(34.0, 4.0)?);
    let round_line = PaintContributionItem::stroke(
        path(vec![
            PathVerb::MoveTo(point(34.0, 24.0)),
            PathVerb::LineTo(point(50.0, 24.0)),
        ]),
        Brush::solid(blue),
        StrokeStyle::new(LogicalLength::new(4.0)?).with_cap(StrokeCap::Round),
    );
    let degenerate = PaintContributionItem::stroke(
        SceneShape::rect(rect(56.0, 8.0, 0.0, 20.0)),
        Brush::solid(Color::WHITE),
        StrokeStyle::new(LogicalLength::new(8.0)?).with_join(StrokeJoin::Round),
    );
    let publication = publication(vec![
        clipped_triangle,
        transformed_ellipse,
        round_line,
        degenerate,
    ]);

    let output = renderer.render_offscreen_publication(&publication, &provider)?;
    let readback = output.readback();
    assert_eq!(pixel(readback, 16, 8), [0x35, 0xB8, 0x68, 0xFF]);
    assert_eq!(
        pixel(readback, 8, 8),
        [0, 0, 0, 0],
        "the rectangular item clip excludes otherwise-covered path fill"
    );
    assert_eq!(pixel(readback, 40, 9), [0xD9, 0x4E, 0x49, 0xFF]);
    assert_eq!(pixel(readback, 42, 24), [0x45, 0x79, 0xD8, 0xFF]);
    assert_eq!(
        pixel(readback, 56, 18),
        [0, 0, 0, 0],
        "zero-extent rectangle stroke remains empty"
    );
    Ok(())
}

#[test]
fn real_gpu_self_overlap_is_one_source_and_rebuilds_after_target_loss() -> Result<(), Box<dyn Error>>
{
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let color = Color::rgba(0xC3, 0x4A, 0x42, 0x80);
    let crossing = path(vec![
        PathVerb::MoveTo(point(8.0, 8.0)),
        PathVerb::LineTo(point(32.0, 32.0)),
        PathVerb::MoveTo(point(32.0, 8.0)),
        PathVerb::LineTo(point(8.0, 32.0)),
    ]);
    let publication = publication(vec![PaintContributionItem::stroke(
        crossing,
        Brush::solid(color),
        StrokeStyle::new(LogicalLength::new(6.0)?).with_cap(StrokeCap::Butt),
    )]);

    let first = renderer.render_offscreen_publication(&publication, &provider)?;
    let arm = pixel(first.readback(), 11, 11);
    let overlap = pixel(first.readback(), 20, 20);
    assert_eq!(
        overlap[3], arm[3],
        "tessellation overlap must not multiply one item's alpha"
    );
    assert!(
        overlap[3].abs_diff(0x80) <= 1,
        "one 0x80-alpha logical source must stay near 0x80, got {}",
        overlap[3]
    );
    for channel in 0..3 {
        assert!(
            overlap[channel].abs_diff(arm[channel]) <= 1,
            "overlap and single-coverage samples must share one source result"
        );
    }

    let first_generation = first.target_generation();
    let first_pixels = first.readback().rgba8_srgb().to_vec();
    assert!(renderer.discard_offscreen_target());
    let rebuilt = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_ne!(rebuilt.target_generation(), first_generation);
    assert_eq!(rebuilt.readback().rgba8_srgb(), first_pixels);
    Ok(())
}

#[test]
fn real_gpu_gradients_match_core_sampling_and_hard_stop_semantics() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let red = Color::rgb(255, 0, 0);
    let blue = Color::rgb(0, 0, 255);

    let hard_stops = gradient_stops(&[(0.0, Color::BLACK), (0.5, red), (0.5, blue), (1.0, Color::WHITE)]);
    let hard_gradient = LinearGradient::new(point(0.5, 0.5), point(64.5, 0.5), hard_stops)
        .unwrap_or_else(|_| unreachable!("fixture hard-stop gradient is valid"));
    let hard_item = PaintContributionItem::fill(
        SceneShape::rect(rect(0.0, 0.0, 64.0, 12.0)),
        Brush::Linear(hard_gradient.clone()),
    );

    let radial_gradient = RadialGradient::new(
        point(48.5, 28.5),
        LogicalLength::new(8.0)?,
        gradient_stops(&[(0.0, Color::BLACK), (1.0, Color::WHITE)]),
    )
    .unwrap_or_else(|_| unreachable!("fixture radial gradient is valid"));
    let radial_item = PaintContributionItem::fill(
        SceneShape::rect(rect(36.0, 16.0, 24.0, 24.0)),
        Brush::Radial(radial_gradient.clone()),
    );

    let alpha_gradient = LinearGradient::new(
        point(0.5, 20.5),
        point(32.5, 20.5),
        gradient_stops(&[
            (0.0, Color::rgb(255, 0, 0)),
            (1.0, Color::rgba(0, 0, 255, 0)),
        ]),
    )
    .unwrap_or_else(|_| unreachable!("fixture alpha gradient is valid"));
    let alpha_item = PaintContributionItem::fill(
        SceneShape::rect(rect(0.0, 16.0, 32.0, 12.0)),
        Brush::Linear(alpha_gradient),
    );

    let publication = publication(vec![hard_item, alpha_item, radial_item]);
    let output = renderer.render_offscreen_publication(&publication, &provider)?;
    let readback = output.readback();

    assert_pixel_near(
        pixel(readback, 8, 4),
        color_bytes(hard_gradient.sample_at(point(8.5, 4.5))),
        1,
    );
    assert_eq!(
        pixel(readback, 32, 4),
        color_bytes(red),
        "the exact hard-stop coordinate keeps the first authored boundary color"
    );
    assert_pixel_near(
        pixel(readback, 33, 4),
        color_bytes(hard_gradient.sample_at(point(33.5, 4.5))),
        1,
    );

    assert_eq!(pixel(readback, 48, 28), color_bytes(Color::BLACK));
    assert_eq!(pixel(readback, 56, 28), color_bytes(Color::WHITE));
    assert_pixel_near(
        pixel(readback, 52, 28),
        color_bytes(radial_gradient.sample_at(point(52.5, 28.5))),
        1,
    );

    let midpoint = pixel(readback, 16, 20);
    assert!(
        midpoint[3].abs_diff(128) <= 1,
        "premultiplied alpha midpoint must remain half-alpha, got {}",
        midpoint[3]
    );
    assert!(
        midpoint[0] >= 186 && midpoint[0] <= 189,
        "opaque-red contribution over transparent target should store half linear red, got {}",
        midpoint[0]
    );
    assert_eq!(
        midpoint[2], 0,
        "transparent blue must not leak color through premultiplied interpolation"
    );
    Ok(())
}

#[test]
fn real_gpu_gradient_transform_clip_stroke_and_rebuild_are_deterministic()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let stops = gradient_stops(&[(0.0, Color::BLACK), (1.0, Color::WHITE)]);
    let fill_gradient = LinearGradient::new(point(0.5, 0.5), point(24.5, 0.5), stops.clone())
        .unwrap_or_else(|_| unreachable!("fixture transformed gradient is valid"));
    let transformed = PaintContributionItem::fill(
        SceneShape::rect(rect(0.0, 0.0, 24.0, 12.0)),
        Brush::Linear(fill_gradient.clone()),
    )
    .with_transform(LogicalTransform::translation(8.0, 32.0)?)
    .with_clip(ContributionClip::identity(SceneShape::rect(rect(
        16.0, 30.0, 12.0, 16.0,
    ))));

    let stroke_gradient = LinearGradient::new(point(36.5, 40.5), point(60.5, 40.5), stops)
        .unwrap_or_else(|_| unreachable!("fixture stroke gradient is valid"));
    let stroke = PaintContributionItem::stroke(
        path(vec![
            PathVerb::MoveTo(point(36.0, 40.0)),
            PathVerb::LineTo(point(60.0, 40.0)),
        ]),
        Brush::Linear(stroke_gradient.clone()),
        StrokeStyle::new(LogicalLength::new(4.0)?).with_cap(StrokeCap::Butt),
    );
    let publication = publication(vec![transformed, stroke]);

    let first = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        pixel(first.readback(), 12, 36),
        [0, 0, 0, 0],
        "the independent owner-local clip excludes transformed primitive coverage"
    );
    assert_pixel_near(
        pixel(first.readback(), 20, 36),
        color_bytes(fill_gradient.sample_at(point(12.5, 4.5))),
        1,
    );
    assert_pixel_near(
        pixel(first.readback(), 48, 40),
        color_bytes(stroke_gradient.sample_at(point(48.5, 40.5))),
        1,
    );

    let first_generation = first.target_generation();
    let first_pixels = first.readback().rgba8_srgb().to_vec();
    assert!(renderer.discard_offscreen_target());
    let rebuilt = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_ne!(rebuilt.target_generation(), first_generation);
    assert_eq!(rebuilt.readback().rgba8_srgb(), first_pixels);
    Ok(())
}

fn color_bytes(color: Color) -> [u8; 4] {
    [color.red(), color.green(), color.blue(), color.alpha()]
}

fn assert_pixel_near(actual: [u8; 4], expected: [u8; 4], tolerance: u8) {
    for channel in 0..4 {
        assert!(
            actual[channel].abs_diff(expected[channel]) <= tolerance,
            "channel {channel} differs: actual={actual:?}, expected={expected:?}, tolerance={tolerance}"
        );
    }
}

fn pixel(readback: &runenui_render_wgpu::OffscreenReadback, x: u32, y: u32) -> [u8; 4] {
    let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
    readback.rgba8_srgb()[index..index + 4]
        .try_into()
        .unwrap_or_else(|_| unreachable!("pixel index is in the fixture target"))
}

fn renderer_or_adapterless() -> Result<Option<Renderer>, Box<dyn Error>> {
    match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => Ok(Some(renderer)),
        Err(RendererInitError::AdapterUnavailable {
            requested,
            compatible_surface_required,
            detail,
        }) => {
            eprintln!(
                "native wgpu solid proof unavailable under {requested:?}; structured adapter failure: {detail}"
            );
            assert_eq!(requested, BackendSelection::AllNative);
            assert!(!compatible_surface_required);
            assert!(!detail.is_empty());
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

struct ThreadWake(thread::Thread);

impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}
