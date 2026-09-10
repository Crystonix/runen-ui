#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{
    sync::Arc,
    task::{Context, Wake, Waker},
    thread,
};

use runenui_core::{
    Brush, Color, ContributionClip, Element, LogicalLength, LogicalPoint, LogicalRect, LogicalSize,
    LogicalTransform, NoHostProtocol, PaintContribution, PaintContributionContext,
    PaintContributionItem, PathFillRule, PathVerb, ResourceRef, ScenePath, SceneShape, StrokeCap,
    StrokeJoin, StrokeStyle, StyleEnvironment, UiApp, Widget, WidgetMeasure,
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
