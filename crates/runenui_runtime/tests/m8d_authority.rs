const TAFFY_LAYOUT_SOURCE: &str = include_str!("../src/surface/taffy_layout.rs");
const PLANNING_SOURCE: &str = include_str!("../src/surface/planning.rs");
const RESOLVE_SOURCE: &str = include_str!("../src/surface/resolve.rs");

#[test]
fn production_text_measurement_and_paint_share_one_retained_artifact_path() {
    assert!(
        TAFFY_LAYOUT_SOURCE
            .contains("let request = TextRequest::new(content, typography, constraints);")
    );
    assert!(
        TAFFY_LAYOUT_SOURCE.contains("match self.text_system.layout_text(&mut state, &request)")
    );
    assert!(TAFFY_LAYOUT_SOURCE.contains("let artifact = outcome.artifact();"));
    assert!(TAFFY_LAYOUT_SOURCE.contains("let text_size = artifact.size();"));
    assert!(TAFFY_LAYOUT_SOURCE.contains("self.final_text_states[index] = Some(state.clone());"));
    assert!(TAFFY_LAYOUT_SOURCE.contains("self.text_layouts[index] = state;"));

    assert!(
        RESOLVE_SOURCE
            .contains("if let Some(artifact) = layout.text_layouts[mounted_preorder].artifact()")
    );
    assert!(RESOLVE_SOURCE.contains(".lease_shaped_run(run.resource_ref())"));
    assert!(
        RESOLVE_SOURCE
            .contains("let item = text_run_item(run, &styles.resolutions[mounted_preorder]);")
    );
    assert_eq!(
        RESOLVE_SOURCE.matches("layout_text(").count(),
        0,
        "paint composition must never reshape or re-line-break text"
    );
}

#[test]
fn runtime_layout_has_one_bounded_taffy_entrypoint_and_no_framework_stabilization_loop() {
    assert_eq!(
        TAFFY_LAYOUT_SOURCE.matches("compute_root_layout(").count(),
        1,
        "one runtime layout transaction enters Taffy exactly once"
    );
    assert!(
        TAFFY_LAYOUT_SOURCE
            .contains("compute_root_layout(&mut kernel, root, available_space(root_constraints));")
    );
    assert_eq!(
        TAFFY_LAYOUT_SOURCE.matches("measure_until_stable").count(),
        0
    );
    assert_eq!(
        TAFFY_LAYOUT_SOURCE.matches("while let Some").count(),
        0,
        "layout authority must not add a framework-owned open-ended stabilization loop"
    );
    assert_eq!(
        PLANNING_SOURCE
            .matches("current.layout = Arc::new(resolve_layout_phase")
            .count(),
        1,
        "a dirty retained surface replaces its layout product through one phase call"
    );
}
