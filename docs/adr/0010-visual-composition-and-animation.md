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

M8 closed the production style/layout/international-text foundation. Runtime already
owns canonical interaction state, deterministic logical time, style resolution,
layout/text feedback, final logical geometry, staged paint/hit/semantic publication,
wake/redraw, and mounted lifetime. The accepted renderer consumes immutable neutral
paint publications and owns only disposable realization.

M9 must add ordinary production visual breadth and motion without creating parallel
state, geometry, scheduling, scene, semantic, or renderer authority. At the accepted
base:

- paint primitives are limited to rectangle fill/stroke, exact-mapped image, and
  shaped text;
- `SceneShape` is rectangle/rounded-rectangle only;
- item-local transform/opacity are paint facts, not node-wide presentation geometry;
- opaque image `ResourceRef` carries no neutral intrinsic-size metadata, so
  contain/cover/nine-slice cannot be renderer policy;
- reduced motion is an explicit preference fact but no animation family exists;
- `MonotonicClock`, deterministic `ManualClock`, canonical wake/redraw, bounded pump,
  trace, and mounted-generation cleanup already exist.

## Relationship to accepted authority

This ADR preserves:

- M3 mounted identity/lifecycle/state/invalidation and one retained runtime tree;
- M4 logical time, scheduling, wake/redraw, one canonical FIFO, and trace;
- M5 semantic identity/publication/action and deterministic public testing;
- M6 immutable renderer-neutral paint/hit publication, exact transform/clip rules,
  `ResourceRef`, retained publication, and staged atomicity;
- M7 concrete renderer/host/resource edges and disposable device caches;
- M8 style precedence/preferences, property-effect classification, runtime-owned
  Taffy layout/final geometry, and exact shaped-text resources.

M10 remains editing/interaction-services authority; M11 controls; M12 virtualization;
M13 broad platform/device profiles.

## Decision

### One renderer-neutral visual/motion flow

```text
authored visual/style/motion intent
    + canonical interaction/application state
    + explicit preferences
    -> runtime-resolved target facts
    -> runtime-owned transition/timeline state
    + one monotonic-time sample
    -> sampled effective style/layout/presentation facts
    -> exact layout/text/paint/hit/focus/semantic dependencies
    -> one staged immutable publication
    -> disposable renderer realization
```

No renderer, dependency, or widget callback owns a competing scene timeline, clock,
scheduler, mounted tree, property cache, or semantic geometry path.

### Visual ownership is split by responsibility

M9 does not create multiple equivalent ways to author the same node-wide visual fact.

**Computed style / runtime-owned node presentation** owns common reusable node-level
policy:

- existing foreground, padding, radius, typography, interaction-state and preference
  resolution;
- background generalized from color-only to a RunenUI-owned brush value;
- optional paint-only outline using a brush plus stroke description;
- ordinary ordered drop-shadow descriptions;
- node/group opacity;
- node presentation transform;
- transition policy per supported property.

The color-only M8 background contract is cleanly replaced rather than kept as a
parallel compatibility API. A solid color remains the trivial brush case. Text
foreground stays a color in initial M9; gradient text is not silently introduced by
background brush generalization.

**Widget owner-local paint contribution** owns content-specific rendering:

- arbitrary accepted shape/path fill and stroke items;
- images and their explicit crop/fit/nine-slice descriptors;
- the already accepted shaped-text resource item;
- item-local transforms, clips, opacity and layer;
- explicit owner-local composition grouping where a custom widget needs it.

A widget paint item transform remains paint-local and does not implicitly move the
widget's hit/focus/semantic geometry. Node-wide presentation motion must use the
runtime presentation-transform property instead.

**Runtime immutable publication** owns composed surface-space facts:

- owner placement plus node presentation transform plus item-local transform;
- snapshot-local composition-group identity/nesting/order;
- composed clips;
- exact scene ordering and resource leases;
- logical primitive/effect bounds and publication damage metadata.

Renderer caches, tessellated vertices, blur targets, gradient uniforms/textures and
device objects remain disposable downstream realization only.

### Public shape/stroke/brush vocabulary remains RunenUI-owned

`runenui_core` owns host/renderer-neutral public values. Dependency path, paint,
color, scene, animation and backend types never enter public protocols.

Initial shapes are rectangle, normalized rounded rectangle, ellipse, and immutable
validated path with move/line/quadratic/cubic/close verbs. Paths use finite logical
`f32` coordinates, explicit contours, and explicit non-zero or even-odd fill rule.
Malformed/non-finite paths reject. Degenerate contours remain only where their
coverage is deterministic. Logical bounds and fill-hit behavior are framework facts,
never renderer-tessellation output.

Strokes own finite non-negative width, cap, join and miter limit. Width zero means no
coverage, never a backend hairline. Invalid join/miter values reject before renderer
realization. Paint is never implicitly converted into a hit target; widgets continue
to author physical hit contributions explicitly using accepted neutral hit shapes.

Initial brushes are solid `Color`, linear gradient and radial gradient. Conic
gradients and arbitrary shader brushes are deferred. Gradient geometry is logical.
Stops have finite offsets in `[0, 1]`, stable nondecreasing order, at least two
entries, and may share offsets for hard stops.

Existing `Color` remains straight-alpha sRGB8 public storage. Gradient evaluation and
continuous color animation use premultiplied linear-sRGB interpolation with
deterministic conversion/clamping. Source-over is the only initial group composition
mode. Additional blend modes require a later explicit neutral contract; backend blend
or shader enums are never widget semantics.

### Images resolve fit before the renderer

`ResourceRef` remains complete opaque image identity. M9 adds a neutral image
descriptor pairing an image `ResourceRef` with immutable intrinsic pixel dimensions
supplied by the logical resource owner. Metadata is descriptive and never a second
identity/cache key.

Image paint owns normalized source crop, logical destination rectangle, fit
(fill/contain/cover/none/scale-down), normalized two-axis alignment, and optional
nine-slice source insets plus explicit logical destination edge widths.

Fit is resolved before renderer realization. Missing/invalid intrinsic metadata
diagnoses; renderer never guesses another fit. Crop, fit, alignment, nine-slice,
layout, raster scale, atlas placement and device state do not remint `ResourceRef`.

Nine-slice source insets cannot overlap after normalization. If destination size is
smaller than opposing logical edge widths, opposing edges are proportionally scaled
so center extent is never negative.

### Composition groups are snapshot-local publication structure

Group opacity cannot be replaced by per-child opacity when children overlap. M9
therefore permits an immutable snapshot-local group table in `PaintScene`.

Initial group composition is source-over only. A group may carry parent group
reference, conjunctive clips, validated group opacity, and accepted ordinary shadow
effects. Paint items reference their containing group.

Group IDs are snapshot-local structure/order only. They have no mounted identity,
lifecycle, reconciliation key, semantic identity, widget state or cross-publication
authority. Runtime derives the table while staging a publication. Renderer offscreen
targets are disposable realization state.

Item opacity remains item-local. Group opacity applies to the composed group result.

### Ordinary shadows have finite neutral coverage

Drop shadows own finite logical offset, non-negative Gaussian sigma, finite signed
spread and straight-alpha `Color`. Negative spread may contract source coverage;
positive spread expands it; collapsed spread yields empty shadow coverage.

To make logical effect bounds deterministic, M9 defines a truncated Gaussian-style
blur with neutral support cutoff at `3 * sigma` from the spread-adjusted shadow
shape on each axis. Renderer kernels may approximate the blur inside that support but
must produce no shadow coverage outside the accepted neutral support. This rule, not
backend kernel choice, determines framework effect/damage bounds.

Shadows expand paint/effect/damage bounds only. They do not alter layout, physical
hit, directional focus or semantic bounds by default.

Arbitrary filter graphs, custom shader callbacks, backend blend programs and public
offscreen handles are not M9 extension mechanisms.

### Extension means explicit neutral contract, not `Any`

M9 adds no generic custom render node/shader escape hatch. Future richer effects must
add an explicit non-exhaustive neutral primitive/effect/resource contract with
validated bounds, diagnostics/fallback behavior and renderer capability semantics.
WGSL, wgpu handles, dependency scene nodes or backend filter enums never become
public framework semantics.

### Presentation geometry is one correlated runtime fact

M9 introduces node-level presentation geometry distinct from widget-authored
item-local paint transforms. A style/motion presentation transform is resolved and
sampled once and composed consistently into paint geometry, physical hit geometry,
directional-focus geometry, semantic bounds, and presentation-relative clips.

The initial public transform is decomposed 2D presentation data: logical translation,
finite scale, finite rotation and normalized origin in the final layout box. Runtime
derives the affine `LogicalTransform`; public animation interpolates components, not
matrix entries.

Hit testing uses the exact transformed hit shape. Directional focus and semantic
bounds use deterministic axis-aligned bounds of transformed authoritative geometry.
Singular transforms have empty physical hit coverage under inherited M6 rules and do
not fall back to untransformed geometry.

Presentation transforms never rewrite layout. Layout-affecting motion feeds sampled
effective layout/style values into the same runtime/Taffy/text path.

### Motion targets are typed derived overrides, never mutation of authored state

A motion target is an explicit RunenUI-owned property identity. Initial targets are:

- supported computed-style visual properties;
- node presentation transform/opacity;
- explicitly accepted numeric layout fields whose endpoints share one domain, such as
  length-to-length or percent-to-percent dimensions/offsets/gaps;
- M8 layout-affecting style values such as padding.

Runtime samples an **effective** style/layout/presentation value for the publication
candidate. It never mutates authored `Element`, mounted authored `LayoutStyle`, theme
recipes, application state or widget state to represent animation progress.

Incompatible layout domains such as `auto`/intrinsic versus numeric values are
discrete. The sampled effective layout is consumed by the accepted single runtime
layout authority; no animation-specific layout engine or renderer geometry shortcut
exists.

Arbitrary paint-item indices are not animation targets in initial M9 because they do
not provide stable framework property identity. Custom widget content changes remain
widget/application state unless represented by an accepted node-level motion target.

### Lyon is the preferred private path/tessellation family

M9 accepts Lyon `1.0.x` as subordinate algorithms/tessellation, subject to exact patch
revalidation in the first dependency-changing implementation PR.

Private use may include `lyon_path`/`lyon_algorithms` for bounds, flattening and hit
algorithms and `lyon_tessellation` for disposable fill/stroke meshes in
`runenui_render_wgpu`. Lyon never owns public path identity, mounted state, scene
order, hit targets, style, animation, resource identity or renderer lifetime.

A0 research found the latest published Lyon meta-crate family at `1.0.19`
(MIT OR Apache-2.0) while upstream `main` remains active and carries newer 1.0.x
subcrate development. Published manifests do not declare a formal `rust-version`;
the first adoption PR must prove the exact patch with RunenUI Rust `1.93.0`, inspect
features/transitives/licenses, and stop if that proof fails.

### Kurbo, Peniko and Vello are not M9 authorities

A0 reviewed the current Linebender stack:

- Kurbo `0.13.1`: mature 2D curves, MIT OR Apache-2.0, MSRV 1.85. Its canonical
  geometry is `f64` and it does not replace tessellation. Do not add a second curve
  model beside Lyon without a concrete missing-algorithm need.
- Peniko `0.6.1`: MIT OR Apache-2.0, MSRV 1.85. Its brush/image/blend vocabulary
  overlaps RunenUI's required public color/resource/style/composition authority, so
  public Peniko types and a parallel Peniko scene model are rejected.
- Vello `0.10.0`: alpha GPU renderer, MIT OR Apache-2.0, MSRV 1.92, currently on
  wgpu `29.0.x` with its own `Scene`/`Renderer`. RunenUI is on wgpu `30.0.x`; wholesale
  Vello adoption would add a second renderer/scene authority and second wgpu major.
  It is rejected for M9.

A future narrow algorithm may be reconsidered only if it does not transfer authority.

### Easing/interpolation remains RunenUI-owned

A0 reviewed `keyframe`, `interpolation`, `enterpolation` and Penner-easing crates.
`keyframe` owns animation-sequence semantics and its current release is old; generic
interpolation crates do not encode RunenUI property compatibility, invalidation,
reduced motion, mounted lifetime or publication atomicity.

The bounded M9 easing layer is built directly: linear plus validated cubic-Bezier
timing curves and named conveniences defined in terms of them. Input/output are
normalized `[0, 1]` scalars with exact endpoints. Easing has no clock/timeline state.

### Style transitions and explicit timelines are distinct

M9 has two motion sources:

1. **Style transitions** animate resolved target changes from ordinary style
   resolution, including canonical hover/focus/active/disabled changes. Interruption
   starts from the currently sampled presented value, so reversal is continuous.
2. **Explicit timelines** are declarative keyframes for motion not caused by a style
   target change. They carry stable authored animation ID, duration, delay, ordered
   normalized keyframes, easing and repeat policy: once, finite count or forever.
   Reversed behavior is authored with reversed keyframes in initial M9.

Transition policy participates in style authoring but is not itself a target visual
property. Timeline descriptions are transient authoring; live state is exact mounted-
generation runtime state.

When an explicit timeline and style transition address the same property, the
explicit timeline is the sole sampled owner while active. A style target may continue
to change underneath it but does not start a competing transition. When the timeline
ends/cancels/removes, runtime resolves the current style target and, if transition
policy applies, transitions from the timeline's final/current sampled value to that
target. Initial M9 has no additive blending of two same-property motion authorities.

Animation completion is deterministic runtime lifecycle/inspection state, not a new
callback/action path. Future completion actions must enter the canonical FIFO through
a separately reviewed public contract.

### Mandatory preference policy remains above motion

M8 mandatory preference policy remains the highest authority for governed style
properties. Style-transition endpoints are the already preference-governed resolved
values. An explicit timeline cannot animate around a mandatory high-contrast or other
preference override: when a property is currently preference-governed, conflicting
explicit timeline samples are suppressed for that property and diagnose through the
canonical motion/style inspection path.

Reduced motion is handled by the dedicated motion policy below. Motion therefore adds
presentation over accepted target history; it does not become a new style-precedence
layer above mandatory preferences.

### Runtime owns live motion and one time sample

`runenui_runtime` owns live transition/timeline records keyed by exact mounted
lifetime and property/animation identity. Only the accepted `MonotonicClock` is used.

One staged surface candidate snapshots one monotonic instant for all motion sampled
into that candidate. No renderer timestamp, wall-clock read, sleep, background
animation thread, second scheduler or per-frame action queue is allowed.

Sampling is history-independent at ordinary frame boundaries: a sample is computed
from retained start/keyframe endpoints plus exact normalized progress at the sampled
instant, never by incrementally integrating or lerping the previous frame sample.
Frame cadence, skipped frames and u8 color rounding therefore cannot change the value
at a given logical instant. The current sampled value becomes a new retained start
only when an accepted interruption/replacement actually creates a new motion
lifetime/segment.

Reconciliation/lifecycle rules:

- unchanged explicit animation ID/spec preserves progress;
- changed spec under same ID cancels/replaces at current runtime time;
- removal cancels;
- mounted replacement/removal/shutdown cancels exact mounted generation;
- style target replacement starts from current sampled presented value;
- zero-duration motion reaches terminal sample in the next staged publication with
  no fake one-frame animation;
- finite timelines complete after final iteration;
- forever timelines never complete naturally.

Completion/cancellation cleanup commits with runtime-owned state/publication planning.
Renderer success/failure is never the clock or completion trigger.

### Existing redraw/wake authority drives frames

Visible active motion keeps redraw authority live. After redraw acknowledgement,
runtime may request another redraw while motion remains active; host presentation or
vsync determines actual frame cadence. Sampling still uses runtime monotonic time.

Headless proof advances `ManualClock`. Runtime does not enqueue a canonical FIFO item
per sample. Existing timers may wake delayed starts/terminal deadlines but remain the
same timer/time authority, not an animation clock.

### Animatability is explicit per property

M9 never infers animatability from generic numeric traits.

Initial continuous families include compatible color/brush/gradient-stop colors,
item/group opacity, presentation translation/scale/rotation/origin, corner radius,
compatible shadow numeric/color fields, padding, and accepted numeric layout values
whose endpoint variants share one interpolation domain.

Initial discrete families include resource identity, path topology/morphing,
incompatible gradient kind/stop structure, image fit/incompatible crop/nine-slice,
`auto`/intrinsic versus numeric layout modes, and typography/font/shaping identity.

Discrete values retain start value until their defined switch boundary, then change
exactly once and invalidate exact dependencies. M9 never substitutes renderer scale
for text-metric animation.

### M8 property-effect classification is extended, not bypassed

The direct-effect model expands beyond `layout`/`paint` as required for M9 while
runtime remains responsible for transitive propagation.

Examples:

- foreground/brush color: paint only;
- opacity/shadow: paint/effect bounds, not layout/hit/semantic;
- presentation transform: paint + hit + focus + semantic presentation geometry,
  without layout;
- padding/numeric size: layout then dependent text/paint/hit/focus/semantics;
- typography: text/layout plus all dependent products.

No sample may use unconditional invalidate-all when a narrower class is sufficient,
and no paint-only classification may leave visible interaction/accessibility stale.

### Reduced motion is deterministic framework policy

`StylePreferences::reduced_motion` remains the sole input. Each motion description has
a bounded reduced-motion strategy with safe defaults:

- ordinary transitions: `SnapToEnd`;
- finite decorative timelines: `SnapToEnd`;
- repeating/forever decorative timelines: `HoldInitial`;
- preserving motion requires explicit `PreserveEssential`.

`SnapToEnd` produces terminal value and completes/cancels live decorative motion.
`HoldInitial` holds the first value, suppresses the live timeline, and issues no
continuous redraw while reduced motion remains active. If reduced motion later turns
off, a suppressed repeating timeline restarts from the preference-change instant; it
does not silently accrue hidden elapsed time. `PreserveEssential` continues normally.

There is no ambient renderer/platform policy and no hidden duration multiplier.
Preference changes invalidate only affected motion and dependent products.

### Staged publication remains atomic

Motion sampling is part of staged surface planning. Recoverable/terminal failure
cannot expose mixed sampled generations such as new paint with old hit/semantic
geometry or commit a sample without corresponding publication facts.

Renderer failure never mutates runtime motion state. Retained retry uses the exact
already-sampled publication. A later runtime publication may sample a later clock
instant; renderer retry need not replay missed intermediate frames.

### Damage/effect bounds are truthful metadata

M6 full-surface damage remains valid. Narrower M9 damage is permitted only after
logical primitive/stroke/shadow/group bounds are deterministic. Damage is publication
metadata, never scene/resource identity.

Logical path/stroke/shadow/group bounds are framework facts. Renderer tessellation/
blur may use conservative subordinate bounds but cannot shrink accepted coverage.

### Diagnostics/trace observe one authority

M9 extends canonical inspection with target changes, timeline/transition start,
replacement, suppression, cancellation, completion, sampled normalized progress,
reduced-motion/preference decision, property-effect classification, group/effect
bounds and renderer-capability rejection. It does not add a mutable second animation
log.

### No new production crate is justified

Initial ownership stays in existing crates:

- `runenui_core`: neutral visual/motion values;
- `runenui_runtime`: live motion, sampling, invalidation, correlated publication;
- `runenui_render_wgpu`: private Lyon/disposable GPU realization;
- `runenui_testing`: public testing ergonomics only if ordinary public seams need it.

No `runenui_animation`/`runenui_vector` crate is created by concept alone. Extract
only for real dependency/optionality/multi-consumer pressure.

### `element.rs` concentration does not block M9A0

Issue #10 identifies historical concentration, but M9 has focused owners in style,
paint, scene geometry, runtime surface and renderer modules. A0 does not authorize a
broad split.

New visual/motion values belong in focused modules and element authoring delegates to
those values. If implementation proves uncontainable change-coupling, schedule a
separate behavior-preserving decomposition. File length alone is not evidence.

## Derived serial implementation sequence

After accepted-main validation of A0, derive child issues in this order:

1. **M9A — production visual/composition vocabulary and real-wgpu realization**:
   style ownership cutover, shapes/paths/strokes/brushes/images/nine-slice/groups/
   shadows, logical bounds/hit, private Lyon adoption, and renderer realization; no
   motion yet.
2. **M9B — deterministic transitions and timelines**: runtime lifecycle/sampling,
   typed effective property overrides, same-property precedence, interpolation/
   effects, redraw, preference policy, reduced motion, and presentation geometry.
3. **M9C — integrated production closure**: public manual-time corpus,
   hover/focus/active transitions, layout/text-affecting motion, real-wgpu visual/
   motion evidence, retained retry/re-realization, and final authority cleanup.

Do not create implementation child issues until A0 is owner-accepted, squash-merged,
and accepted-main validated.

## Consequences

- Common production desktop/game visuals are neutral-contract expressible.
- M8 hover/focus/active state can transition without duplicate interaction state.
- Animated presentation geometry stays physically and semantically truthful.
- Motion never mutates authored state or creates a parallel layout authority.
- Public APIs are insulated from Lyon/Peniko/Vello/backend churn.
- Renderer complexity grows for paths, gradients, groups and shadows, but remains
  disposable realization state.
- Deferred breadth includes arbitrary shaders/filters, additional blend modes, path
  morphing, conic gradients, additive same-property animation layers, generic
  completion callbacks and visual animation editors.

## Validation obligations

The M9 conformance matrix owns permanent observations. Each implementation slice must
start from accepted `main`, revalidate exact dependency versions/features/MSRV/
licenses when first adopting Lyon, run canonical `cargo validate`, pass exact-head
hosted CI, receive complete-diff cold review with zero unresolved debt, and undergo
bounded accepted-main/current-truth reconciliation before rows are promoted.
