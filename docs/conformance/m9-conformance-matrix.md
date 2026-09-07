# M9 Conformance Matrix

> **Category:** Target architecture
>
> **Status:** M9 target; implementation blocked pending accepted delivery
>
> **Milestone:** M9
>
> **Reviewed baseline:** `68808fb1dd5bfbd644a74fcfe916870428bcdd59`
>
> This matrix becomes the permanent M9 observable/proof contract only after the
> M9A0 package containing it is owner-accepted, squash-merged, and accepted-main
> validated. Every M9 row is initially `blocked`; target acceptance does not claim
> production implementation.

[ADR 0010](../adr/0010-visual-composition-and-animation.md) owns M9 architecture.
M3 owns mounted identity/lifecycle/invalidation; M4 owns logical time, scheduling,
wake/redraw, and trace; M5 owns semantic identity/publication/testing; M6 owns the
renderer-neutral paint/hit publication model and `ResourceRef`; M7 owns concrete
renderer/host/resource edges; M8 owns production style/layout/text and current
property-effect classification. This matrix adds only new M9 observations.

```text
25 total unique rows
0 owner-accepted
0 implementation-complete
0 proof-complete
25 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

All rows are `Required`. Serial M9 delivery is M9A visual/composition production,
M9B deterministic transitions/timelines, then M9C integrated closure.

A dependency-owned public path/paint vocabulary, retained second scene/tree,
renderer-owned animation clock/timeline, per-frame action queue, stale hit/semantic
geometry under visual motion, ambient preference policy, custom backend shader escape
hatch, software expected renderer, or compatibility shim preserving replaced pre-1.0
visual authority cannot satisfy M9.

## M9A — production visual/composition vocabulary and renderer realization

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M9VIS-01 | Public visual/composition vocabulary is RunenUI-owned and responsibility-normalized: common node background-brush/outline/shadow/opacity/presentation policy resolves through computed style/runtime; arbitrary shape/path/image content remains owner-local paint contribution; composed transforms/groups/effect bounds remain immutable runtime publication. No Lyon, Kurbo, Peniko, Vello, wgpu, WGSL, native, mounted, or dependency scene type becomes public authority. | Public-API/style/paint/publication ownership corpus | Duplicate-authority, forbidden-type/public-dependency, item-transform-as-node-presentation, and second-scene audit | Visual schema/provenance/ownership report | M9A | blocked | Required |
| M9VIS-02 | Validated rect/rounded-rect/ellipse/path geometry has finite logical coordinates, explicit contour/fill-rule semantics, deterministic logical bounds, and fill-hit behavior independent of renderer tessellation; paint never implicitly manufactures a physical hit target. | Shape/path/bounds/hit corpus | Non-finite/malformed path, raster/tessellation-derived authority, and paint-derived-hit audit | Path validation/bounds/hit records | M9A | blocked | Required |
| M9VIS-03 | Stroke semantics explicitly preserve finite non-negative width, cap, join, and miter behavior; zero width has no coverage and is never a backend hairline. | Stroke geometry corpus | Backend-default/hairline/invalid-miter corpus | Stroke normalization/bounds records | M9A | blocked | Required |
| M9VIS-04 | Solid, linear-gradient, and radial-gradient brushes preserve validated logical geometry, stable nondecreasing stops, hard stops, straight-alpha sRGB8 inputs, and premultiplied linear-sRGB interpolation without backend-specific color policy; the M8 color-only background is cleanly replaced by brush-valued background rather than retained as parallel authority. | Brush/gradient/color/style-cutover corpus | Unsorted/non-finite stop, hidden color-space, backend-brush, and duplicate-background audit | Gradient normalization/interpolation/style provenance records | M9A | blocked | Required |
| M9VIS-05 | Image fit/crop/alignment is resolved from exact opaque image `ResourceRef` plus immutable neutral intrinsic-size metadata; fill/contain/cover/none/scale-down never depend on renderer guesses or remint resource identity. | Image descriptor/fit/crop corpus | Missing-metadata fallback, renderer-fit, scale/device-bound identity audit | Image geometry/resource correlation records | M9A | blocked | Required |
| M9VIS-06 | Nine-slice imagery preserves normalized immutable source insets, explicit logical destination edge widths, deterministic undersized-destination normalization, and the same complete image `ResourceRef`. | Nine-slice geometry corpus | Negative-center/overlapping-source/reminted-resource corpus | Slice normalization/placement records | M9A | blocked | Required |
| M9VIS-07 | Immutable snapshot-local composition groups preserve nesting, clips, exact ordered membership, and source-over group opacity without acquiring mounted identity, lifecycle, reconciliation, semantic identity, or cross-publication authority; group opacity is applied to the composed group result rather than rewritten as per-child alpha. | Group composition corpus | Per-child-opacity substitution, backend blend-policy, and retained-second-tree audit | Group structure/composition records | M9A | blocked | Required |
| M9VIS-08 | Ordinary drop shadows preserve exact neutral offset, non-negative sigma, finite signed spread, color, and deterministic support truncated to `3 * sigma` from spread-adjusted coverage; shadows expand paint/damage only and do not silently alter layout/hit/focus/semantic bounds. | Shadow/effect-bounds/support corpus | Infinite/backend-defined support, invalid/collapsed spread, clipped-shadow, and semantic/layout inflation audit | Effect-bound/realization diagnostics | M9A | blocked | Required |
| M9VIS-09 | The extension boundary remains explicit non-exhaustive neutral primitives/effects/resources; arbitrary shader/filter/backend callbacks, `Any` render nodes, device handles, or dependency-owned scene subtrees are not public escape hatches. | Extension/capability corpus | Shader/WGSL/wgpu-handle/custom-scene authority audit | Unsupported-capability diagnostics | M9A | blocked | Required |
| M9VIS-10 | Lyon `1.0.x`, when adopted, is private subordinate path/tessellation machinery only; `runenui_render_wgpu` owns disposable tessellation/GPU/group/shadow caches and can reconstruct them from retained neutral publication/resource facts after cache/device loss. | Exact-dependency + real-wgpu realization/re-realization corpus | Public-Lyon-type, retained-Lyon-tree, second-renderer, Vello/Peniko scene, and cache-as-authority audit | Dependency/version/MSRV/license + renderer cache records | M9A | blocked | Required |

## M9B — deterministic transitions and timelines

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M9MOTION-01 | All animation sampling uses the accepted runtime `MonotonicClock`; one staged surface candidate observes one exact monotonic instant with no wall-clock read, renderer timestamp, sleep, background animation thread, or second clock. | Manual-clock/same-sample corpus | Wall-clock/renderer-time/second-clock audit | Sample instant records | M9B | blocked | Required |
| M9MOTION-02 | Style transitions start only from canonical resolved target changes, including hover/focus/active/disabled state, and interruption/reversal starts from the currently sampled presented value rather than stale prior target state. | Interaction/style-transition corpus | Second interaction-state, target-jump, and stale-start-value corpus | Target/start/sample provenance | M9B | blocked | Required |
| M9MOTION-03 | Explicit timelines are declarative authored values with stable animation identity, ordered normalized keyframes, duration/delay/repeat policy, and exact mounted-generation lifecycle; unchanged identity/spec preserves progress while replacement/removal/unmount/shutdown cancel deterministically. For one property, an active explicit timeline is the sole sampled owner: style targets may change underneath it but cannot start a competing transition, and timeline termination resumes from the timeline's current/final sample toward the then-current style target under normal transition policy. | Reconciliation/timeline-lifetime/same-property-precedence corpus | Widget-local hidden timer/state-machine, stale-generation, additive same-property animation, and competing transition audit | Start/replace/cancel/lifetime/precedence records | M9B | blocked | Required |
| M9MOTION-04 | Linear/cubic-Bezier easing and timeline normalization have exact endpoints, deterministic delay/zero-duration/iteration/completion semantics, finite checked time arithmetic, and forever timelines never claim natural completion. Ordinary samples are pure functions of retained segment endpoints/keyframes plus exact normalized progress at the sampled instant; they never incrementally integrate previous frame samples, so frame cadence/skips/rounding cannot change a value at the same logical instant. | Easing/timing/iteration/history-independence corpus | NaN/out-of-range/overflow/phantom-frame, ambiguous-completion, frame-rate-dependent integration, and accumulated-rounding corpus | Normalized progress/endpoint/sample/completion records | M9B | blocked | Required |
| M9MOTION-05 | Motion targets are explicit typed derived overrides, never mutation of authored/application/widget state. Compatible color/brush/opacity/presentation-transform/radius/shadow/padding and accepted same-domain numeric layout values interpolate continuously; resource/path/incompatible-gradient/auto-intrinsic/typography identity changes follow explicit discrete rules. Sampled effective layout values feed the accepted runtime/Taffy/text authority, and arbitrary paint-item indices are not stable animation targets. | Property-target/compatibility/interpolation/effective-layout corpus | Generic-Lerp, authored-state mutation, animation-specific layout engine, path-morph/resource-crossfade/text-scale, and paint-index target audit | Motion-target/interpolation/effective-value records | M9B | blocked | Required |
| M9MOTION-06 | Sampled property changes use exact direct-effect classification and transitive invalidation: paint-only motion avoids layout/text, presentation motion updates paint/hit/focus/semantic geometry without relayout, layout motion recomputes layout/dependents, and text-metric changes use the accepted text/layout authority. | Differential animated-invalidation corpus | Under-invalidation, invalidate-all, renderer-text-scale, and stale-product audit | Property-effect/dirty-phase records | M9B | blocked | Required |
| M9MOTION-07 | Mandatory M8 preference overrides remain above motion. Reduced-motion behavior derives only from explicit `StylePreferences::reduced_motion` plus bounded RunenUI policy: transitions/finite decorative timelines default to snap-to-end; repeating decorative timelines default to hold-initial, suppress their live timeline and continuous redraw, and restart from the preference-change instant rather than accruing hidden elapsed time when reduced motion is later disabled; preserved essential motion requires explicit declaration. Conflicting explicit motion cannot animate around a currently mandatory high-contrast/property override. | Preference-precedence/motion-policy/suppression-restart corpus | Ambient platform/renderer policy, motion-above-preference, hidden speed multiplier/time accrual, redraw-while-held, and ignored-preference audit | Preference/motion decision/suppression/restart/cancellation records | M9B | blocked | Required |
| M9MOTION-08 | Active visible motion uses existing redraw/wake authority; redraw acknowledgement may request another frame while motion remains active, but runtime does not enqueue one canonical FIFO action per sample or invent a fixed framework frame rate. | Redraw/vsync/manual-time corpus | Per-frame FIFO, busy settle-loop, hidden 60-Hz timer, and lost-redraw audit | Redraw/wake/active-motion records | M9B | blocked | Required |
| M9MOTION-09 | Motion sampling/transition cleanup participates in staged publication atomically; recoverable/terminal failure cannot publish mixed sampled generations, and renderer success/failure never advances or mutates runtime motion authority. | Publication failure/retry corpus | Partial paint-hit-semantic sample, renderer-driven completion, and retry-resample audit | Candidate/sample/commit/retry correlation | M9B | blocked | Required |
| M9MOTION-10 | Animation inspection extends the canonical runtime diagnostics/trace with target/start/replacement/suppression/restart/cancellation/completion/sample/preference facts without creating a second mutable animation log or requiring application actions to implement debug/tween traits. | Trace/inspection corpus | Parallel event-log/debug-trait/action-channel audit | Canonical motion trace records | M9B | blocked | Required |

## M9C — integrated production closure

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M9INTEG-01 | One runtime-sampled node presentation transform is composed consistently into visible paint, physical hit testing, directional-focus geometry, semantic bounds, and presentation-relative clips; hit uses exact transformed shape, focus/semantics use deterministic axis-aligned bounds of transformed authoritative geometry, and singular transforms have inherited empty-hit behavior rather than stale layout fallback. | Transform/paint-hit-focus-semantic correlation corpus | Paint-only transform/stale hit/accessibility/untransformed fallback audit | Node/sample/geometry correlation records | M9C | blocked | Required |
| M9INTEG-02 | Deterministic public/headless proof advances `ManualClock` explicitly and observes exact intermediate/end/replacement/preference/layout/text/semantic results through ordinary public runtime contracts without sleeps, private expected runtime, alternate scene engine, or software expected renderer. | Public manual-time integration corpus | Sleep/wall-clock/private-model/alternate-engine audit | Fixture/time/publication diagnostics | M9C | blocked | Required |
| M9INTEG-03 | Real-wgpu proof renders the same accepted neutral paths/strokes/gradients/images/nine-slice/groups/shadows and representative transition samples, including retained-publication retry and disposable re-realization, without renderer-owned style/timeline/hit/semantic authority. | Real-wgpu visual/motion/contact-sheet corpus | Vello/second-renderer, software expected renderer, renderer timeline, and resource-rebinding audit | Runtime/scene/renderer correlation records | M9C | blocked | Required |
| M9INTEG-04 | Representative canonical hover/focus/active transitions visibly and deterministically exercise the M8 interaction/style path plus M9 transition path; no application-maintained hover state, widget timer, or showcase-only animation authority is required. | Interactive-state transition corpus | Duplicate hover state/showcase timer/direct renderer animation audit | Interaction-style-motion provenance | M9C | blocked | Required |
| M9INTEG-05 | Final M9 authority cleanup removes/revises bounded M6/M8 assumptions superseded by accepted M9 production visuals/motion, with current docs/API exposing one truthful path and no compatibility alias, hidden renderer fallback, or obsolete proof authority preserved for convenience. | Source/API/current-truth cleanup corpus | Duplicate primitive/composition/motion/compatibility-authority audit | Repository authority/deprecation audit | M9C | blocked | Required |

## Closure rule

M9 is conformance-complete only when all 25 rows are `owner-accepted` on accepted
default branch. Each serial implementation slice must be separately owner-accepted,
squash-merged, accepted-main validated, and followed by bounded current-truth/
conformance reconciliation before its rows become `owner-accepted`. M9 closes only
after final integrated reconciliation and accepted-main validation. Later M10/M11
work may consume the accepted visual/motion contracts but must not move their clock,
scene, property, hit, semantic, or renderer authority elsewhere.
