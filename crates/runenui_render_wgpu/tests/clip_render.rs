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
    PaintContributionItem, PathFillRule, PathVerb, ResourceRef, ScenePath, SceneShape,
    StyleEnvironment, UiApp, Widget, WidgetMeasure,
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
            "generic-clip proof unexpectedly requested a resource",
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

fn path_rect(rect: LogicalRect) -> SceneShape {
    SceneShape::path(
        ScenePath::new(
            vec![
                PathVerb::MoveTo(point(rect.x(), rect.y())),
                PathVerb::LineTo(point(rect.max_x(), rect.y())),
                PathVerb::LineTo(point(rect.max_x(), rect.max_y())),
                PathVerb::LineTo(point(rect.x(), rect.max_y())),
                PathVerb::Close,
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("fixture path clip is structurally valid")),
    )
}

fn publication(items: Vec<PaintContributionItem>) -> PaintPublication {
    let mut runtime = AppRuntime::<FixtureApp>::mount(items);
    let environment = StyleEnvironment::default();
    let logical_size = LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
        .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::tight(logical_size))
        .with_raster_scale(RasterScale::ONE);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
        .paint_publication()
        .clone()
}

fn clipped_item(clips: impl IntoIterator<Item = ContributionClip>) -> PaintContributionItem {
    clips.into_iter().fold(
        PaintContributionItem::fill(
            SceneShape::rect(rect(
                0.0,
                0.0,
                f32::from(SURFACE_WIDTH),
                f32::from(SURFACE_HEIGHT),
            )),
            Brush::solid(Color::rgb(0x36, 0xB5, 0x6B)),
        ),
        PaintContributionItem::with_clip,
    )
}

#[test]
fn real_gpu_ellipse_and_path_clips_are_conjunctive_order_invariant_and_rebuildable()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;

    let ellipse = ContributionClip::new(
        SceneShape::ellipse(rect(4.0, 6.0, 40.0, 28.0)),
        LogicalTransform::translation(4.0, 2.0)?,
    );
    let path = ContributionClip::identity(path_rect(rect(20.0, 10.0, 32.0, 24.0)));

    let forward = publication(vec![clipped_item([ellipse.clone(), path.clone()])]);
    let reversed = publication(vec![clipped_item([path, ellipse])]);

    let first = renderer.render_offscreen_publication(&forward, &provider)?;
    assert_eq!(
        pixel(first.readback(), 28, 22),
        [0x36, 0xB5, 0x6B, 0xFF],
        "the point inside both generic clips remains covered"
    );
    assert_eq!(
        pixel(first.readback(), 16, 22),
        [0, 0, 0, 0],
        "ellipse-only coverage is removed by the path clip"
    );
    assert_eq!(
        pixel(first.readback(), 50, 12),
        [0, 0, 0, 0],
        "path-only coverage is removed by the transformed ellipse clip"
    );

    let forward_pixels = first.readback().rgba8_srgb().to_vec();
    let reversed_output = renderer.render_offscreen_publication(&reversed, &provider)?;
    assert_eq!(
        reversed_output.readback().rgba8_srgb(),
        forward_pixels,
        "conjunctive clip coverage cannot depend on authored clip-list order"
    );

    assert!(renderer.discard_offscreen_target());
    let rebuilt = renderer.render_offscreen_publication(&forward, &provider)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_eq!(
        rebuilt.readback().rgba8_srgb(),
        forward_pixels,
        "generic clip realization reconstructs identically after target loss"
    );
    Ok(())
}

#[test]
fn real_gpu_empty_and_singular_generic_clips_erase_coverage() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;

    let empty = ContributionClip::identity(SceneShape::ellipse(rect(8.0, 8.0, 0.0, 20.0)));
    let singular = ContributionClip::new(
        path_rect(rect(8.0, 8.0, 32.0, 24.0)),
        LogicalTransform::try_new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)?,
    );
    let output = renderer.render_offscreen_publication(
        &publication(vec![clipped_item([empty]), clipped_item([singular])]),
        &provider,
    )?;

    assert!(
        output
            .readback()
            .rgba8_srgb()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| *pixel == [0, 0, 0, 0]),
        "empty and singular clips must not fall back to untransformed/full coverage"
    );
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
                "native wgpu generic-clip proof unavailable under {requested:?}; structured adapter failure: {detail}"
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
