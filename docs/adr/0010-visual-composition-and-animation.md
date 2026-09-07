# ADR 0010: Visual composition and deterministic animation

> **Category:** ADR
>
> **Status:** Accepted target architecture on exact-head owner acceptance
>
> **Decision date:** 2026-09-07
>
> **Milestone:** M9
>
> **Reviewed baseline:** `68808fb1dd5bfbd644a74fcfe916870428bcdd59`
>
> **Acceptance:** this ADR becomes the accepted M9 target architecture only after
> the exact M9A0 package containing it is explicitly accepted by the repository
> owner, squash-merged, and accepted-main validated. Acceptance freezes target
> decisions; it does not claim M9 production implementation or dependency adoption.

## Context

M8 closed the production style/layout/international-text foundation. The accepted
runtime already owns canonical interaction state, deterministic logical time,
style resolution, layout/text feedback, final logical geometry, staged paint/hit/
semantic publication, wake/redraw, and mounted lifetime. The accepted renderer
already consumes immutable renderer-neutral paint publications and owns only
disposable GPU/resource realization.

M9 must add normal production visual breadth and motion without turning any of those
accepted facts into duplicated authorities. In particular:

- the current paint primitive set is intentionally limited to rectangle fill/stroke,
  exact-mapped images, and shaped text;
- `SceneShape` covers only rectangles and rounded rectangles;
- item-local transforms and item opacity are self-contained paint facts, not a
  node-wide presentation-geometry authority;
- images expose opaque logical `ResourceRef` identity but no renderer-neutral
  intrinsic-size metadata, so contain/cover/nine-slice cannot truthfully be left to
  a renderer-specific guess;
- reduced motion is already an explicit preference fact, but M8 has no animation
  property family;
- the runtime already provides `MonotonicClock`, deterministic `ManualClock`,
  canonical wake/redraw, bounded pumping, and mounted-generation cleanup.

## Relationship to accepted authority

This ADR preserves:

- M3 mounted identity/lifecycle/state/invalidation and one retained runtime tree;
- M4 one deterministic logical-time/work/wake/redraw authority and canonical trace;
- M5 semantic identity/publication/action and deterministic public testing;
- M6 immutable renderer-neutral paint/hit scenes, exact transform/clip semantics,
  `ResourceRef`, retained publication, and staged atomicity;
- M7 the concrete wgpu renderer/host/resource edges and disposable device caches;
- M8 canonical style precedence/preferences, property-effect classification,
  runtime-owned Taffy layout/final geometry, and the exact shaped-text resource path.

M10 remains the owner of editing/selection/clipboard and broader interaction services;
M11 owns standard controls; M12 owns virtualization; M13 owns broad platform/device
profiles. M9 does not acquire those responsibilities indirectly through animation.

## Decision

### One renderer-neutral visual and motion flow

```text
authored visual/style/motion intent
    + canonical interaction/application state
    + explicit preferences
    -> runtime-resolved target visual facts
    -> runtime-owned live transition/timeline state
    + one monotonic-time sample
    -> sampled presentation/style facts
    -> exact dependent layout/text/paint/hit/focus/semantic work
    -> one staged immutable publication
    -> disposable renderer realization
```

No renderer, dependency, or widget callback owns a competing scene timeline, clock,
scheduler, mounted tree, semantic geometry path, or property cache.

### RunenUI owns the public visual vocabulary

Public M9 vocabulary remains `runenui_core`-owned and backend-neutral. Dependency
path/paint/color/scene types are not public protocol types.

The initial production shape vocabulary is:

- rectangle;
- rounded rectangle with the existing normalized-corner rule;
- ellipse;
- immutable validated path made from move, line, quadratic, cubic, and close verbs.

Paths use finite logical `f32` coordinates, explicit contour boundaries, and an
explicit non-zero/even-odd fill rule. Empty/degenerate contours remain representable
where their meaning is deterministic, but malformed/non-finite verbs are rejected.
Logical path bounds and hit behavior are framework facts; renderer tessellation is
not an authority for either.

The production stroke contract explicitly owns width, cap, join, and miter limit.
Stroke width is finite and non-negative. Zero width means no coverage, never a
backend hairline. Miter-limit and degenerate-join behavior are deterministic and
validated before renderer realization.

### Brushes and color/composition semantics

A RunenUI-owned brush is initially one of:

- solid `Color`;
- linear gradient;
- radial gradient.

Conic gradients and arbitrary shader brushes are not part of the initial M9
contract. Gradient geometry is expressed in logical coordinates. Stops are finite
normalized offsets in `[0, 1]`, stored in stable authored order after validation;
nondecreasing offsets are required and equal offsets are permitted for hard stops.
At least two stops are required.

The accepted `Color` remains straight-alpha sRGB8 public storage. Gradient and color
animation interpolation is defined in premultiplied linear-sRGB space and converted
back to the public representation with deterministic clamping/rounding. Rendering
uses source-over unless an explicitly accepted composition group says otherwise.
Backend blend/shader enums never become widget semantics.

### Images carry enough neutral geometry to make fit deterministic

`ResourceRef` remains the complete opaque logical image identity. M9 adds a neutral
image descriptor that pairs an image `ResourceRef` with immutable intrinsic pixel
size metadata supplied by the logical resource owner. Intrinsic size is descriptive
metadata, not a second resource identity and not renderer cache state.

Image paint owns:

- normalized source rectangle/crop;
- logical destination rectangle;
- fit mode: fill, contain, cover, none, or scale-down;
- normalized two-axis alignment;
- optional nine-slice source insets plus explicit logical destination edge widths.

Contain/cover/scale-down geometry is resolved from the descriptor before renderer
realization. Invalid or unavailable intrinsic metadata diagnoses explicitly; the
renderer must not silently choose another fit. Source crop, fit, alignment, nine-
slice, layout extent, raster scale, atlas placement, and device state do not remint
`ResourceRef`.

Nine-slice source insets are normalized to immutable source-image dimensions and
must not overlap after normalization. Destination edge widths are logical lengths;
when the destination is smaller than opposing edges, the edges are proportionally
normalized rather than creating negative center geometry.

### Composition groups are immutable publication structure, not a retained UI tree

Correct group opacity and future bounded offscreen effects cannot be represented as
independent per-item alpha when children overlap. M9 therefore permits an immutable
snapshot-local composition-group table in `PaintScene`.

A group may carry:

- parent snapshot-local group reference;
- conjunctive clip(s);
- validated group opacity;
- a bounded composition mode required by accepted M9 behavior;
- ordinary renderer-neutral effects accepted by this ADR.

Paint items reference their containing group. Group identifiers are snapshot-local
ordering/structure values only: they have no mounted identity, lifecycle, widget
state, semantic identity, reconciliation key, or cross-publication authority.
Runtime derives the group structure while staging one publication; the renderer may
materialize temporary offscreen targets but cannot retain a second framework scene.

Per-item opacity remains valid for truly item-local alpha. Group opacity is applied
after rendering group contents into the group result; it must not be rewritten as
per-child opacity when that changes compositing.

### Ordinary shadows are bounded renderer-neutral effects

M9 accepts ordinary drop shadows expressed by finite logical offset, non-negative
Gaussian sigma, non-negative spread, and straight-alpha `Color`. Shadow coverage
expands logical effect/damage bounds but does not enlarge layout, hit, focus, or
semantic bounds by default.

The renderer owns blur kernels, temporary targets, sampling strategy, and quality
tradeoffs that preserve the neutral shadow contract. An arbitrary filter graph,
custom shader callback, backend blend program, or public offscreen texture handle is
not an M9 extension mechanism.

### The renderer-neutral extension boundary is deliberate refusal, not `Any`

M9 does not add a generic custom-shader/custom-render-node escape hatch. The
extension boundary is the accepted non-exhaustive RunenUI scene/resource vocabulary:
future richer effects require a new explicit neutral primitive/effect/resource
contract with deterministic bounds, semantics, fallback/diagnostic behavior, and
renderer capability requirements.

This preserves multiple-renderer viability and prevents renderer handles, WGSL,
dependency scene nodes, or backend blend/filter enums from becoming framework API.

### Presentation geometry is a runtime-wide correlated fact

M9 introduces node-level presentation geometry distinct from widget-authored
item-local paint transforms. A style/motion presentation transform is resolved and
sampled once by runtime and then composed consistently into:

- paint geometry;
- hit geometry;
- directional-focus geometry;
- semantic bounds;
- any clip geometry whose accepted coordinate space is presentation-relative.

A visually translated/scaled/rotated control therefore cannot remain interactive or
accessibility-addressed at stale layout coordinates.

The initial presentation transform is a validated decomposed 2D value with logical
translation, finite scale, finite rotation, and a normalized transform origin within
the final layout box. Runtime derives the final affine `LogicalTransform`. Component
interpolation is deterministic; matrix-entry interpolation is not the public motion
contract. Singular sampled transforms have empty physical hit coverage according to
the inherited M6 rule and diagnose where required.

Presentation transforms do not rewrite layout geometry. Layout-affecting motion uses
actual layout properties and the accepted layout/text authority instead.

### Adopt Lyon privately for path algorithms/tessellation; do not expose it

M9 accepts the Lyon `1.0.x` family as the preferred subordinate path algorithm and
GPU-mesh tessellation family, subject to exact-patch revalidation in the first
implementation PR that adds it.

Intended use is private:

- `lyon_path` / `lyon_algorithms` may support exact path bounds, flattening, and hit
  algorithms behind RunenUI-owned values where that reduces correctness risk;
- `lyon_tessellation` may be used by `runenui_render_wgpu` for disposable fill/stroke
  tessellation.

Lyon never owns public path identity, mounted state, scene ordering, hit target
identity, style, animation, resource identity, or renderer lifetime. Tessellated
vertices are disposable renderer data.

At A0 research time the latest published Lyon meta-crate family is `1.0.19`
(MIT OR Apache-2.0); upstream `main` is actively maintained and already carries
newer 1.0.x subcrate development. Published manifests do not declare a formal
`rust-version`, so the implementation PR must explicitly prove the chosen exact
patch against RunenUI Rust `1.93.0`, inspect transitive dependencies/features, and
pin/reject it if that proof fails.

### Kurbo, Peniko, and Vello are not adopted as M9 authorities

A0 reviewed the current Linebender stack:

- Kurbo `0.13.1` is mature, MIT OR Apache-2.0, MSRV 1.85, and useful 2D curve
  infrastructure. Its canonical geometry is `f64` and it does not remove the need
  for a renderer tessellator. M9 does not add a second private curve model alongside
  Lyon without a concrete missing-algorithm need.
- Peniko `0.6.1` is MIT OR Apache-2.0 with MSRV 1.85 and provides solid/gradient/
  image/blend styling types. Those types overlap exactly with RunenUI's required
  public color/resource/style/composition semantics, so adopting Peniko as public
  vocabulary or a parallel scene model is rejected.
- Vello `0.10.0` is an alpha GPU compute renderer, MIT OR Apache-2.0, MSRV 1.92,
  currently tied to wgpu `29.0.x`, and exposes its own `Scene`/`Renderer` model.
  RunenUI's accepted renderer is already on wgpu `30.0.x`; adopting Vello wholesale
  would add a second renderer/scene authority and a second wgpu major. It is rejected
  for M9. Future evidence may reconsider a narrowly isolated algorithm if it no
  longer transfers authority.

### Easing/interpolation semantics are small enough to remain RunenUI-owned

A0 reviewed generic animation/easing crates including `keyframe`, `interpolation`,
`enterpolation`, and Penner-easing ports. `keyframe`'s current release is old and
owns animation-sequence semantics; generic interpolation crates do not encode
RunenUI property compatibility, invalidation, reduced-motion, mounted lifetime, or
publication atomicity.

M9 therefore builds the bounded easing/interpolation layer directly. Initial easing
is linear plus validated cubic Bézier timing curves and named convenience values
defined in terms of those curves. Timing input and output are normalized `[0, 1]`
scalars; endpoint values are exact. Easing does not own clock/timeline state.

### Style transitions and explicit timelines are separate descriptions

M9 supports two motion sources:

1. **Style transitions** animate a resolved property target change caused by normal
   style resolution, including canonical hover/focus/active/disabled changes. A
   transition starts from the currently sampled presented value, not necessarily the
   previous target, so interruption/reversal is continuous.
2. **Explicit timelines** are host-neutral declarative keyframe descriptions for
   bounded visual/property motion not caused by a style-target change. They carry a
   stable authored animation ID, duration, delay, ordered normalized keyframes,
   easing per segment or timeline, and repeat policy (`once`, finite count, or
   forever). Reversed motion is authored by reversed keyframes rather than a second
   direction state machine in the initial M9 contract.

Transition policy participates in style authoring but is not itself a target visual
property. Explicit timeline descriptions remain transient authoring input; the live
runtime state they create is mounted-generation-owned.

M9 completion is deterministic runtime lifecycle/inspection state. It does not
create a second callback/action channel. If a later product requires application
actions on animation completion, those actions must enter the accepted canonical FIFO
through a separately reviewed public contract.

### Runtime owns live animation state and one clock sample

`runenui_runtime` owns live transition/timeline records keyed by exact mounted
lifetime and property/animation identity. It uses only the accepted
`MonotonicClock`.

For each staged surface candidate runtime snapshots one monotonic instant and uses
that same instant for all motion sampled into that candidate. No renderer timestamp,
wall-clock read, sleep, background animation thread, second scheduler, or per-frame
action queue is allowed.

Reconciliation semantics are exact:

- unchanged explicit animation identity/specification preserves the live timeline;
- changed specification under the same ID cancels/replaces the prior generation at
  the current runtime time;
- removal cancels it;
- mounted replacement/removal and shutdown cancel the exact mounted generation;
- a style target change replaces a same-property transition from the current sampled
  value;
- zero-duration transitions/timelines resolve synchronously to their terminal sample
  in the next staged publication and do not create a fake one-frame animation.

Finite timelines complete after their final iteration. Forever timelines never
complete naturally. Completion/cancellation cleanup is committed atomically with
runtime-owned state/publication planning; renderer success/failure is not the clock
or completion trigger.

### Active motion uses existing redraw/wake authority

Active visible motion keeps the accepted redraw authority live. After a redraw is
acknowledged, runtime may issue the next redraw request while visible animation
remains active; the host's normal presentation/vsync loop chooses actual frame
cadence. Sampling always uses runtime monotonic time.

Headless tests advance `ManualClock` explicitly. The runtime must not create one
canonical FIFO envelope per animation sample. Timers may be used for future delayed
start/end readiness where appropriate, but they remain the accepted timer authority
and are not a second animation clock.

### Animatable properties have an explicit compatibility matrix

M9 never infers animatability from `Add`, `Lerp`, `Into<f32>`, or matching Rust enum
variants. Each property has an explicit interpolation and downstream-effect rule.

Initial continuous families include:

- `Color` and compatible brush colors/gradient stops;
- scene/group opacity;
- presentation translation/scale/rotation/origin;
- corner radius and shadow numeric/color fields;
- compatible logical lengths/spacing/layout dimensions where both endpoints share
  one interpolation domain.

Initial discrete families include:

- resource identity;
- image fit mode and incompatible crop/nine-slice structure;
- path topology/morphing;
- incompatible gradient kind/stop structure;
- `auto`/intrinsic versus numeric layout modes;
- typography/font/shaping identity changes.

A discrete property remains at the start value until the defined transition boundary
and then switches exactly to the target; its downstream invalidation occurs at that
switch. M9 does not fake text-metric animation with renderer scale. If typography or
another text-metric property changes, the accepted text/layout authority recomputes
at the exact switch/sample required by its explicit rule.

### Property effects extend M8 classification rather than bypass it

The M8 property-effect model expands beyond `layout`/`paint` so runtime can represent
the exact direct dependency classes required by M9, including presentation/hit/
semantic consequences. The model remains a direct-effect classification; runtime
propagates transitive dependencies.

Examples:

- foreground/brush color: paint only;
- group opacity/shadow: paint plus effect/damage bounds, not layout/hit/semantic;
- presentation transform: paint + hit + focus + semantic presentation geometry,
  without relayout;
- padding or numeric layout size: layout, then dependent text/paint/hit/focus/
  semantics;
- typography: text measurement/layout plus all dependent products.

No animation sample may use unconditional invalidate-all when a narrower accepted
dependency class is sufficient, and no paint-only classification may leave visible
interaction/accessibility geometry stale.

### Reduced motion is explicit deterministic policy

`StylePreferences::reduced_motion` remains the input authority. Every motion
description has an explicit reduced-motion strategy selected from a bounded
RunenUI-owned policy, with safe framework defaults:

- ordinary style transitions default to `SnapToEnd`;
- finite decorative timelines default to `SnapToEnd`;
- repeating/forever decorative timelines default to `HoldInitial`;
- preserving motion while reduced motion is active requires an explicit
  `PreserveEssential` declaration.

The runtime enforces the strategy before starting/replacing live motion. There is no
renderer/platform-global override and no hidden duration multiplier. Preference
changes invalidate/cancel/re-sample only affected motion and dependent products.

### Staged publication remains atomic

Motion sampling is part of staged surface planning. A recoverable/terminal failure
cannot expose a partial combination such as new paint with old hit/semantic geometry
or a newly committed transition sample without its corresponding publication facts.

Renderer failure never mutates runtime motion state. Retained publication retry uses
the exact already-sampled publication. Later successful runtime publication may
sample a later clock instant; renderer retry is not required to replay every missed
intermediate frame.

### Damage and effect bounds become truthful but remain publication metadata

M6 permits full-surface damage. M9 may introduce narrower damage only after logical
primitive/stroke/shadow/group bounds are deterministic. Damage remains publication
metadata and never part of scene/resource identity.

Path/stroke/shadow bounds used for inspection/damage are framework-computed logical
facts. Renderer tessellation/blur extent may be conservative subordinate detail but
must never shrink coverage below the accepted logical effect bound.

### Diagnostics and trace observe one authority

M9 adds inspectable records for target changes, transition/timeline start/replacement/
cancellation/completion, sampled normalized progress, reduced-motion decision,
property effect classification, scene-group/effect bounds, and renderer capability
rejection. These extend existing runtime/scene/trace inspection; they do not create a
second event log or mutable animation debugger authority.

### No new production crate is justified by A0

M9 begins inside existing ownership:

- `runenui_core`: neutral visual/motion description values;
- `runenui_runtime`: live timeline/transition state, sampling, dependency propagation,
  correlated geometry/publication;
- `runenui_render_wgpu`: Lyon-backed/disposable realization, gradient/shadow/group GPU
  work and caches;
- `runenui_testing`: public deterministic ergonomics only if ordinary public seams
  need wrappers.

A separate `runenui_animation` or `runenui_vector` crate is not justified. Extract
only if later dependency/optionality/multi-consumer pressure gives Cargo a real
boundary to enforce.

### Existing `element.rs` concentration is not an M9 prerequisite

Issue #10 identifies real historical concentration, but current M9 responsibilities
already have coherent owners in `style`, `paint`, `scene_geometry`, runtime surface,
and renderer modules. A0 does not authorize a broad `element.rs` split.

New visual/motion value types should live in focused modules and element authoring
should delegate rather than accumulate independent implementation logic. If an
implementation slice proves change-coupling that cannot be contained that way, a
separate behavior-preserving internal decomposition may be scheduled; file length
alone is not evidence.

## Derived serial implementation sequence

After A0 is accepted-main validated, derive child issues in this order:

1. **M9A — production visual/composition vocabulary and real-wgpu realization**:
   shapes/paths/strokes/brushes/images/nine-slice/groups/shadows, logical bounds/hit
   correlation, private Lyon adoption, and renderer realization with no motion yet.
2. **M9B — deterministic transitions and timelines**: runtime-owned transition/
   timeline lifecycle, sampling, property interpolation/effects, redraw behavior,
   reduced-motion, and exact presentation-geometry correlation.
3. **M9C — integrated production closure**: public headless/manual-time corpus,
   representative hover/focus/active transitions, responsive/layout/text-affecting
   motion, real-wgpu visual/motion evidence, retained retry/re-realization, and final
   obsolete-authority cleanup.

Do not create those implementation issues until this A0 package is owner-accepted,
squash-merged, and accepted-main validated.

## Consequences

- Common application/game visuals become expressible without backend-specific widget
  semantics.
- Hover/focus/active style state from M8 can transition smoothly without a second
  interaction state machine.
- Animated presentation geometry remains physically and semantically truthful.
- Public APIs remain stable against renderer/library churn because Lyon is private and
  Peniko/Vello types do not leak.
- Renderer implementation grows in complexity for paths, gradients, groups, and
  shadows, but that complexity remains disposable realization state.
- Some breadth is deliberately deferred: arbitrary shaders/filters, path morphing,
  conic gradients, generic animation-completion callbacks, and design-tool timelines.

## Validation obligations

M9 implementation cannot claim conformance from this ADR alone. Permanent observable
requirements live in the M9 conformance matrix. Each implementation slice must use a
fresh branch from accepted `main`, prove exact dependency patch/features/MSRV/license
when first adopting Lyon, run canonical `cargo validate`, pass exact-head hosted CI,
receive complete-diff cold review with zero unresolved review debt, then undergo the
same separate accepted-main/current-truth reconciliation discipline used by M8.
