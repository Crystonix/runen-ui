#![allow(refining_impl_trait)]

use runenui_core::{
    Element, FontFamilyName, GenericFontFamily, LogicalLength, NoHostProtocol, StyleEnvironment,
    UiApp, View, text,
};
use runenui_runtime::{
    AppRuntime, AxisConstraints, LayoutConstraints, RasterScale, SurfaceBuildContext,
    SurfacePublication,
};

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));
const ARABIC: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/RunenUIFixtureArabic-Regular.ttf"
));
const DEVANAGARI: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/RunenUIFixtureDevanagari-Regular.ttf"
));

const CONTENT: &str = "AV O8 responsive production text سلام क्षि wraps through one layout authority";

struct IntegrationApp;

impl UiApp for IntegrationApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        text(CONTENT).id("copy").key("copy").into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn register_controlled_fonts(runtime: &mut AppRuntime<IntegrationApp>) {
    for bytes in [CANTARELL, ARABIC, DEVANAGARI] {
        assert!(runtime.register_text_font_bytes(bytes.to_vec()).is_ok());
    }
    let families = [
        FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!()),
        FontFamilyName::new("RunenUI Fixture Arabic").unwrap_or_else(|_| unreachable!()),
        FontFamilyName::new("RunenUI Fixture Devanagari").unwrap_or_else(|_| unreachable!()),
    ];
    assert!(
        runtime
            .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &families)
            .is_ok()
    );
}

fn publish(
    runtime: &mut AppRuntime<IntegrationApp>,
    environment: &StyleEnvironment,
    width: u16,
    raster_scale: RasterScale,
) -> SurfacePublication {
    let constraints = LayoutConstraints::new(
        AxisConstraints::tight(LogicalLength::from(width)),
        AxisConstraints::unbounded(),
    );
    runtime
        .publish_surface(
            &SurfaceBuildContext::new(environment, constraints).with_raster_scale(raster_scale),
        )
        .unwrap_or_else(|_| unreachable!("controlled M8D publication is admitted"))
}

fn shaped_refs(publication: &SurfacePublication) -> Vec<runenui_core::ResourceRef> {
    publication
        .paint_publication()
        .scene()
        .items()
        .iter()
        .filter_map(|item| {
            item.primitive()
                .as_shaped_text_run()
                .map(|run| run.resource_ref().clone())
        })
        .collect()
}

fn assert_publication_correlation(publication: &SurfacePublication) {
    let frame = publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("text root is published"));
    let layout = publication
        .layout_report()
        .root()
        .unwrap_or_else(|| unreachable!("text root has layout facts"));
    assert_eq!(frame.id(), layout.id());
    assert_eq!(
        frame.authored_id().map(runenui_core::ElementId::as_str),
        Some("copy")
    );
    assert_eq!(
        layout.authored_id().map(runenui_core::ElementId::as_str),
        Some("copy")
    );
    assert_eq!(frame.bounds().width(), layout.layout_extent().width());
    assert_eq!(frame.bounds().height(), layout.layout_extent().height());

    let semantics = publication.semantic_publication().snapshot();
    assert_eq!(semantics.nodes().len(), 1);
    let semantic = &semantics.nodes()[0];
    assert_eq!(semantic.name(), Some(CONTENT));
    assert_eq!(semantic.text().and_then(runenui_core::SemanticText::as_plain), Some(CONTENT));
    assert_eq!(semantic.bounds(), frame.bounds());

    let refs = shaped_refs(publication);
    assert!(!refs.is_empty());
    for resource_ref in refs {
        let resource = publication
            .paint_publication()
            .scene()
            .shaped_text_resource(&resource_ref)
            .unwrap_or_else(|| unreachable!("paint retains its exact shaped resource"));
        assert_eq!(resource.resource_ref(), &resource_ref);
        assert!(!resource.glyphs().is_empty());
        assert!(
            [CANTARELL, ARABIC, DEVANAGARI]
                .iter()
                .any(|bytes| *bytes == resource.font().bytes()),
            "public deterministic corpus must not fall through to ambient fonts"
        );
    }
}

#[test]
fn public_m8d_corpus_correlates_layout_text_paint_and_semantics() {
    let mut runtime = AppRuntime::<IntegrationApp>::mount(());
    register_controlled_fonts(&mut runtime);
    let environment = StyleEnvironment::default();

    let wide = publish(&mut runtime, &environment, 420, RasterScale::ONE);
    assert_publication_correlation(&wide);
    let wide_height = wide.frame().root().unwrap_or_else(|| unreachable!()).bounds().height();

    let narrow = publish(&mut runtime, &environment, 120, RasterScale::ONE);
    assert_publication_correlation(&narrow);
    let narrow_height = narrow
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .bounds()
        .height();
    assert!(
        narrow_height > wide_height,
        "available width must feed the production text line-break measurement path"
    );

    let repeated = publish(&mut runtime, &environment, 120, RasterScale::ONE);
    assert_publication_correlation(&repeated);
    assert_eq!(shaped_refs(&repeated), shaped_refs(&narrow));
    assert!(runtime.last_surface_phase_report().executed().is_empty());

    let two_x = publish(
        &mut runtime,
        &environment,
        120,
        RasterScale::new(2.0).unwrap_or_else(|_| unreachable!()),
    );
    assert_publication_correlation(&two_x);
    assert_eq!(shaped_refs(&two_x), shaped_refs(&narrow));
    assert_eq!(two_x.frame(), narrow.frame());
    assert_eq!(two_x.layout_report(), narrow.layout_report());
    assert_eq!(
        two_x.semantic_publication().snapshot(),
        narrow.semantic_publication().snapshot()
    );
}
