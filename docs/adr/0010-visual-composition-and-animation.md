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
wake/redraw, trace, and mounted lifetime. The accepted renderer consumes immutable
renderer-neutral paint publications and owns only disposable realization.

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
- M6 immutable renderer-neutral paint/hit publication, exact item/clip transform
  semantics, `ResourceRef`, retained publication, and staged atomicity;
- M7 concrete renderer/host/resource edges and disposable device caches;
- M8 style precedence/preferences, property-effect classification, runtime-owned
  Taffy layout/final geometry, and exact shaped-text resources.

M10 remains editing/interaction-services authority; M11 controls; M12 virtualization;
M13 broad platform/device profiles. M9 does not acquire those responsibilities
indirectly through visual effects or animation.

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

A widget paint-item transform remains paint-local and does not implicitly move the
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

### Public shape/path semantics remain RunenUI-owned

`runenui_core` owns host/renderer-neutral public values. Dependency path, paint,
color, scene, animation and backend types never enter public protocols.

Initial shapes are rectangle, normalized rounded rectangle, ellipse, and immutable
validated path using move, line, quadratic, cubic, and close verbs.

Paths use finite logical `f32` coordinates, explicit contour boundaries, and an
explicit non-zero or even-odd fill rule. A non-empty contour begins with `move`;
segments before a contour start reject; `close` closes only the current contour;
malformed/non-finite paths reject. Degenerate contours remain representable only
where their coverage is deterministic. Logical fill bounds and fill-hit behavior are
framework facts, never renderer-tessellation output.

Paint never implicitly manufactures a physical hit target. Widgets continue to author
physical hit contributions explicitly using accepted neutral hit shapes.

### Stroke placement, caps, joins, and bounds are explicit

The initial stroke contract is **centered on the authoritative path/shape boundary**.
Half of a positive stroke width lies on each side of the centerline. M9 does not add
inside/outside stroke placement; adding either later requires a new explicit neutral
contract.

Stroke width is finite and non-negative. Width zero has no coverage and never means a
backend hairline. Initial caps are butt, round, and square. Initial joins are miter,
bevel, and round. The miter limit is a finite ratio of miter length to stroke width,
must be at least `1`, and a join whose required ratio exceeds the limit falls back to
bevel deterministically. Degenerate joins/caps and closed-contour seams follow the
same neutral geometry rules rather than backend defaults.

Logical stroke bounds include the complete centered stroke, cap, and join coverage.
Renderer tessellation may be conservative but cannot shrink the accepted logical
coverage.

### Brushes, color, opacity, and composition are explicit

Initial brushes are solid `Color`, linear gradient, and radial gradient. Conic
gradients and arbitrary shader brushes are deferred.

Brush geometry is expressed in primitive-local logical coordinates and therefore
follows the same item/node/owner transform chain as the primitive. A linear gradient
is defined by two distinct finite points: stop `0` is sampled at the start point and
stop `1` at the end point, with projection clamped to `[0, 1]`. Equal start/end points
reject. An initial radial gradient is concentric: one finite center and one finite
strictly-positive logical radius; stop `0` is the center and stop `1` the radius.
Focal/conic/two-circle radial variants are later breadth rather than renderer policy.

Gradient stops have finite offsets in `[0, 1]`, stable nondecreasing order, at least
two entries, and may share offsets for hard stops. At equal offsets the later authored
stop wins on the increasing-coordinate side, making a deterministic discontinuity.
Outside the first/last stop range the endpoint color is extended.

Existing `Color` remains straight-alpha sRGB8 public storage. Gradient evaluation and
continuous color animation use premultiplied linear-sRGB interpolation with
deterministic conversion, clamping, and rounding back to the public representation.

Item opacity is validated item-local alpha. Group opacity is validated alpha applied
once to the already-composed group result. **Source-over is the only initial group
composition mode.** Additional blend modes require a later explicit neutral contract;
backend blend/shader enums never become widget semantics.

### Images resolve exact fit/crop/nine-slice geometry before the renderer

`ResourceRef` remains complete opaque image identity. M9 adds a neutral image
descriptor pairing an image `ResourceRef` with immutable non-zero intrinsic pixel
dimensions supplied by the logical resource owner. Intrinsic dimensions are
metadata, never a second identity/cache handle. For fit calculations those dimensions
form the source's intrinsic logical aspect/extent; raster scale remains separate.

Ordinary image painting has one normalized source crop and one logical destination
rectangle. Crop coordinates are finite normalized fractions in `[0, 1]`; width and
height must be positive and the rectangle must remain within the source. Fit is
computed against the cropped source aspect ratio. Alignment is a finite normalized
pair in `[0, 1]`: `0` means start/top, `0.5` center, `1` end/bottom.

For cropped source extent `(sw, sh)` and destination extent `(dw, dh)`:

- `fill` maps the complete crop to the complete destination with independent axis
  scales;
- `contain` uses uniform scale `min(dw / sw, dh / sh)` and alignment places the
  resulting unused destination space;
- `cover` uses uniform scale `max(dw / sw, dh / sh)` and alignment selects which
  overflow of the scaled source is clipped by the destination;
- `none` uses scale `1` in logical units and alignment places/crops that intrinsic
  extent against the destination;
- `scale-down` is `none` when the unscaled crop fits both destination axes and
  otherwise `contain`.

Zero destination extent produces empty image coverage. Missing/invalid intrinsic
metadata diagnoses; renderer never guesses another fit. Runtime/publication resolve
the exact final source and destination geometry before realization.

Nine-slice is a distinct initial image mode rather than an ambiguous combination with
ordinary fit. It owns a source rectangle, four source insets, one destination
rectangle, and four logical destination edge widths. Source insets are validated
against immutable source dimensions and cannot overlap. If destination size is
smaller than opposing destination edge widths, those opposing widths are
proportionally scaled so the center never becomes negative. Corners are mapped to
corners; edges stretch only along their long axis; the center stretches on both axes.
Tiling/repeat modes are not initial M9 semantics.

Crop, fit, alignment, nine-slice, layout, raster scale, atlas placement, device state,
and resolved source/destination geometry do not remint `ResourceRef`.

### Transform order and clip order are frozen

M9 preserves the accepted M6 distinction between an item's local transform and each
clip's independent owner-local transform while adding one node presentation transform.
The exact composition order is:

1. primitive-local geometry is transformed by the item-local transform;
2. node presentation transform is applied in owner-local space;
3. final layout owner-placement translation maps owner-local space to surface space.

The public decomposed node presentation transform evaluates about its resolved origin
in this component order:

1. scale;
2. rotate;
3. presentation translation.

Presentation translation itself is not scaled or rotated. Runtime then composes the
result with final layout owner placement. Matrix-entry interpolation is not public
motion semantics.

A contribution clip keeps its own accepted clip-local-to-owner transform; it is not
implicitly multiplied by the paint item's item-local transform. Runtime composes each
clip with the same node presentation transform and owner placement used for the
owner's presentation geometry. Clips are conjunctive and retain authored order.

Nested composition-group clips accumulate outer-to-inner. Item clips constrain the
item before it contributes to its group. A group's children are source-over composed;
its ordinary shadow effects are derived from that composed child coverage; group
clips constrain the complete group result including group effects; group opacity is
then applied once; the resulting group is source-over composed into its parent. This
order is renderer-neutral publication semantics, not an offscreen implementation
recipe.

A non-invertible transform retains inherited empty physical-hit behavior. Non-finite
composition rejects/diagnoses rather than silently dropping transforms or clips.

### Composition groups are snapshot-local publication structure

Group opacity cannot be replaced by per-child opacity when children overlap. M9
therefore permits an immutable snapshot-local group table in `PaintScene`.

A group may carry parent snapshot-local group reference, ordered conjunctive clips,
validated group opacity, and ordered ordinary drop shadows accepted by this ADR.
Paint items reference their containing group.

Group IDs are snapshot-local structure/order only. They have no mounted identity,
lifecycle, reconciliation key, semantic identity, widget state, or cross-publication
authority. Runtime derives the table while staging a publication. Renderer offscreen
targets are disposable realization state.

### Ordinary shadows have finite neutral coverage

Drop shadows own finite logical offset, non-negative Gaussian sigma, finite signed
spread, and straight-alpha `Color`. Negative spread may contract source coverage;
positive spread expands it; collapsed spread yields empty shadow coverage.

To make logical effect bounds deterministic, M9 defines a truncated Gaussian-style
blur with neutral support cutoff at `3 * sigma` from the spread-adjusted shadow shape
on each axis. Renderer kernels may approximate the blur inside that support but must
produce no shadow coverage outside the accepted neutral support. This rule, not
backend kernel choice, determines framework effect/damage bounds.

Shadows expand paint/effect/damage bounds only. They do not alter layout, physical
hit, directional focus, or semantic bounds by default.

Arbitrary filter graphs, custom shader callbacks, backend blend programs and public
offscreen handles are not M9 extension mechanisms.

### Extension means explicit neutral contract, not `Any`

M9 adds no generic custom render node/shader escape hatch. Future richer effects must
add an explicit non-exhaustive neutral primitive/effect/resource contract with
validated bounds, diagnostics/fallback behavior, and renderer capability semantics.
WGSL, wgpu handles, dependency scene nodes or backend filter enums never become
public framework semantics.

### Presentation geometry is one correlated runtime fact

M9 introduces node-level presentation geometry distinct from widget-authored
item-local paint transforms. A style/motion presentation transform is resolved and
sampled once and composed consistently into paint geometry, physical hit geometry,
directional-focus geometry, semantic bounds, and presentation-relative clip geometry.

The initial public transform is decomposed 2D presentation data: logical translation,
finite scale, finite rotation, and normalized origin in the final layout box. Runtime
derives the affine `LogicalTransform` using the frozen order above.

Hit testing uses the exact transformed hit shape. Directional focus and semantic
bounds use deterministic axis-aligned bounds of transformed authoritative geometry.
Singular transforms have empty physical hit coverage under inherited M6 rules and do
not fall back to untransformed geometry.

Presentation transforms never rewrite layout. Layout-affecting motion feeds sampled
effective layout/style values into the same runtime/Taffy/text path.

### Motion targets are typed derived overrides, never mutation of authored state

A motion target is an explicit RunenUI-owned property identity. Initial targets are
supported computed-style visual properties, node presentation transform/opacity,
explicitly accepted numeric layout fields whose endpoints share one domain, and M8
layout-affecting style values such as padding.

Runtime samples an **effective** style/layout/presentation value for the publication
candidate. It never mutates authored `Element`, mounted authored `LayoutStyle`, theme
recipes, application state, or widget state to represent animation progress.

Incompatible layout domains such as `auto`/intrinsic versus numeric values are
discrete. The sampled effective layout is consumed by the accepted single runtime
layout authority; no animation-specific layout engine or renderer geometry shortcut
exists.

Arbitrary paint-item indices are not animation targets in initial M9 because they do
not provide stable framework property identity. Custom widget content changes remain
widget/application state unless represented by an accepted node-level motion target.

### Style transitions and explicit timelines are distinct

M9 has two motion sources:

1. **Style transitions** animate resolved target changes from ordinary style
   resolution, including canonical hover/focus/active/disabled changes. Interruption
   starts from the currently sampled presented value, so reversal is continuous.
2. **Explicit timelines** are declarative keyframes for motion not caused by a style
   target change. They carry stable authored animation ID and exact timing/keyframe
   data defined below.

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

### Timeline, keyframe, repeat, start, and completion semantics are exact

A transition/timeline duration and delay are checked finite `Duration` values. Delay
holds the retained start/first value. A style transition's start instant is the one
candidate monotonic instant at which its new governed target first commits into live
motion state. An explicit timeline's start instant is the candidate instant at which
its new mounted-generation specification first commits. Failed staging does not
consume a start instant or secretly advance a new motion lifetime.

An explicit timeline contains at least two keyframes. Keyframe offsets are finite,
strictly increasing normalized values; the first is exactly `0`, the last exactly
`1`. Every segment has one easing function, defaulting to linear. Duplicate offsets
are rejected rather than given implicit discontinuity semantics. A desired jump is a
discrete target/property transition at an explicit boundary, not duplicate keyframes.

`duration` is the duration of one iteration. Delay applies once before the first
iteration. Repeat is either one iteration, an explicit non-zero finite total iteration
count, or forever. Initial M9 has no implicit autoreverse/direction mode; reversed
motion is authored with reversed keyframe values.

Within an iteration, normalized progress is `(sample_time - active_iteration_start) /
duration`, clamped to `[0, 1]`, then mapped to the containing keyframe segment and its
easing. At an exact non-final iteration boundary the new iteration begins at keyframe
`0`; immediately before that boundary the previous iteration approaches keyframe `1`.
A final finite boundary samples keyframe `1` and completes.

Zero duration with zero delay reaches the terminal value in the same successfully
committed staged candidate that starts the motion; it never creates a fake one-frame
animation. Zero duration with positive delay holds the start value until the first
candidate at or after the delay deadline, then commits the terminal value and
completion atomically.

Replacement uses one exact candidate time: runtime samples the old motion at that
instant, records cancellation/replacement of the old lifetime, and uses that exact
sample as the new transition start where continuity is required. Canonical motion
inspection orders cancellation/replacement before the replacement start record.
Natural completion is recorded with the candidate that commits the exact terminal
sample and cleanup. No application action is emitted by initial M9 completion.

Mounted removal/replacement cancels only the exact mounted generation; shutdown
cancels every remaining live motion before final closure. Forever timelines never
naturally complete.

### Mandatory preference policy remains above motion

M8 mandatory preference policy remains the highest authority for governed style
properties. Style-transition endpoints are already preference-governed resolved
values. An explicit timeline cannot animate around a mandatory high-contrast or other
preference override: when a property is currently preference-governed, conflicting
explicit timeline samples are suppressed for that property and diagnose through the
canonical motion/style inspection path.

Reduced motion is handled by the dedicated motion policy below. Motion therefore adds
presentation over accepted target history; it does not become a style-precedence
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
Frame cadence, skipped frames and color rounding therefore cannot change the value at
a given logical instant. The current sampled value becomes a new retained start only
when an accepted interruption/replacement actually creates a new motion lifetime or
segment.

### Easing/interpolation remains RunenUI-owned

Initial easing is linear plus validated cubic-Bezier timing curves and named
conveniences defined in terms of those curves. Cubic-Bezier timing control-point x
coordinates are finite and constrained to `[0, 1]` so normalized input has one
well-defined output; y may be finite outside `[0, 1]` and the resulting eased progress
is not silently reinterpreted by renderers. Exact timeline endpoints still sample the
exact property endpoints.

M9 never infers animatability from generic numeric traits. Initial continuous
families include compatible color/brush/gradient-stop colors, item/group opacity,
presentation translation/scale/rotation/origin, corner radius, compatible shadow
numeric/color fields, padding, and accepted numeric layout values whose endpoint
variants share one interpolation domain.

Initial discrete families include resource identity, path topology/morphing,
incompatible gradient kind/stop structure, image fit/incompatible crop/nine-slice,
`auto`/intrinsic versus numeric layout modes, and typography/font/shaping identity.

Discrete values retain the start value until their defined switch boundary, then
change exactly once and invalidate exact dependencies. M9 never substitutes renderer
scale for text-metric animation.

### Existing redraw/wake authority drives frames

Visible active motion keeps redraw authority live. After redraw acknowledgement,
runtime may request another redraw while motion remains active; host presentation or
vsync determines actual frame cadence. Sampling still uses runtime monotonic time.

Headless proof advances `ManualClock`. Runtime does not enqueue a canonical FIFO item
per sample. Existing timers may wake delayed starts/terminal deadlines but remain the
same timer/time authority, not an animation clock.

### Property effects extend M8 classification, not bypass it

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

### Cache compatibility and revisions do not become a second motion authority

M9 adds no global frame counter, renderer-time revision, or animation sample ID that
can replace semantic content identity.

A live motion record is identified by exact mounted generation plus target property
and source identity:

- a style transition is additionally compatible with its resolved target/provenance,
  transition specification, preference facts, retained start value, and start instant;
- an explicit timeline is additionally compatible with its authored animation ID,
  exact timeline specification, retained lifecycle generation, and start instant.

Replacement/cancellation/completion changes live-motion compatibility. Ordinary
sampling does **not** mint a new motion identity.

For one candidate, the sampled effective value participates only in compatibility for
stages that consume that property. Layout/text caches remain reusable when their exact
effective inputs are unchanged; paint/hit/focus/semantic caches remain reusable when
their exact relevant effective facts are unchanged. A later clock instant mapping to
the same effective value does not force unrelated recomputation merely because time
advanced.

Inherited product/publication revision rules remain authoritative. In particular, the
M6 renderer tuple `(PaintScene content, logical size, RasterScale)` reuses its exact
paint publication/revision when unchanged and allocates a checked next paint revision
when that tuple changes; semantic-only or hit-only changes do not allocate paint
revision. M9 does not add a time-only exception to those rules.

Active motion may keep future redraw requested even when consecutive samples quantize
to equal published content. Redraw demand is scheduling state, not scene/resource
identity.

Static path/brush/image/group facts are keyed by their structural neutral content and,
where applicable, the complete opaque `ResourceRef`. Fit/crop/nine-slice, raster
scale, device identity, atlas placement, tessellation buffers, gradient GPU data,
blur targets, and motion time never remint logical resource identity. Renderer caches
may add device/raster/quality-specific subordinate keys privately and must rebuild
after cache/device loss from retained neutral publication/resource facts.

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

Logical path/stroke/shadow/group bounds are framework facts. Renderer tessellation or
blur may use conservative subordinate bounds but cannot shrink accepted coverage.

### Diagnostics/trace observe one authority

M9 extends canonical inspection with target changes, timeline/transition start,
replacement, suppression, cancellation, completion, sampled normalized progress,
reduced-motion/preference decision, property-effect/cache decision, group/effect
bounds and renderer-capability rejection. It does not add a mutable second animation
log or cache ledger.

## Adopt-versus-build research and decisions

A0 research is a decision input, not a runtime source of truth. Exact dependency
patches are revalidated when first added. The snapshot below records the serious
candidates inspected on 2026-09-07 against RunenUI Rust `1.93.0` and the accepted
wgpu `30.0.0` renderer.

| Candidate | Observed upstream state and build surface | Authority fit | M9 decision |
|---|---|---|---|
| Lyon | Actively maintained; meta crate `1.0.19`; current upstream subcrates include `lyon_path 1.0.19`, `lyon_algorithms 1.0.21`, `lyon_tessellation 1.0.22`; MIT OR Apache-2.0; inspected manifests do not declare `rust-version`; selected path/algorithm/tessellation crates use `std` by default and add Lyon geometry plus small numeric helpers (`num-traits`, `float_next_after`) with serde optional; no wgpu dependency. This is a low-to-moderate isolated compile surface compared with a renderer adoption; the first dependency PR measures/records actual clean-build impact. | Purpose-built path algorithms/tessellation without retained UI/renderer-scene authority; exact patch MSRV remains to be proven. | **Adopt privately, by exact required subcrates rather than the meta crate by default.** Public RunenUI path/stroke types remain authoritative. First dependency PR proves exact patches, features, transitives, licenses, Rust 1.93, and shared corpus/benchmark. |
| Kurbo | `0.13.1`; MSRV `1.85`; Apache-2.0 OR MIT; actively maintained; default `std`; no mandatory graphics backend; dependencies are small curve/numeric containers plus optional interoperability/serde/schema features. | Strong `f64` curve algorithms, but using it beside Lyon creates a second private geometry model/conversion path without removing tessellation. | **Do not adopt initially.** Reconsider only for one concrete missing algorithm that justifies the conversion boundary. |
| Peniko | `0.6.1`; MSRV `1.85`; Apache-2.0 OR MIT; actively maintained; default `std`; depends on Kurbo, color, smallvec and resource-handle vocabulary; no wgpu dependency. | Brush/gradient/image/blend/resource types overlap the exact public authority M9 must own. | **Reject as public or parallel paint vocabulary.** Do not add merely to avoid RunenUI-owned types/adapters. |
| Vello | `0.10.0`; MSRV `1.89`; Apache-2.0 OR MIT; actively maintained; broad scene/renderer stack with Peniko/Kurbo and current wgpu/naga `29.0.3`. | Large renderer/build/dependency surface; wholesale adoption creates a second scene/renderer authority and second wgpu major beside RunenUI wgpu 30. | **Reject wholesale for M9.** Reconsider only a narrowly isolated algorithm that transfers no retained scene/timeline/renderer authority. |
| `keyframe` | Published `1.1.1` (2022); small crate using `num-traits` plus optional/default mint support; docs expose easing and an `AnimationSequence` that tracks keyframes/time. | Sequence/time semantics directly overlap runtime-owned timeline authority while still lacking RunenUI property compatibility, invalidation, mounted lifetime, preferences and publication rules. | **Reject sequence adoption; build bounded RunenUI easing/timeline semantics.** |
| `interpolation` | `0.3.0` (2023); tiny dependency-free generic interpolation crate with `Lerp`/easing/cubic-Bezier APIs. | Cheap build cost, but generic interpolation encourages inferred animatability and supplies none of RunenUI lifecycle/invalidation policy. | **Do not adopt initially.** The bounded property-specific layer is smaller than the authority adapter it would still require. |
| `enterpolation` | `0.3.0` (2025); MIT OR Apache-2.0; substantially broader interpolation/extrapolation/B-spline/NURBS surface with `num-traits` and `topology-traits`; feature-gated methods and optional serde. | Far broader mathematical surface than M9's initial linear/cubic-Bezier timing/property interpolation and still does not own framework lifecycle/invalidation correctly. | **Do not adopt initially.** |

The selected Lyon boundary is subordinate algorithms/data realization only. Lyon never
owns public path identity, mounted state, scene order, hit target identity, style,
animation, resource identity, cache compatibility, or renderer lifetime. A Lyon path
algorithm is usable only through a RunenUI adapter whose tolerance/degenerate behavior
is fixed by the accepted logical contract and conformance corpus; a dependency default
cannot silently define public hit/bounds semantics.

## Package/module boundaries

Initial ownership stays in existing crates:

- `runenui_core`: neutral visual/motion description values;
- `runenui_runtime`: live motion, sampling, invalidation/cache compatibility,
  correlated geometry/publication;
- `runenui_render_wgpu`: private Lyon-backed disposable tessellation and GPU
  realization;
- `runenui_testing`: public deterministic ergonomics only if ordinary public seams
  need wrappers.

No `runenui_animation`, `runenui_vector`, or `runenui_style` production crate is
created by concept alone. Extract only for a real ownership/dependency/optionality/
multiple-consumer boundary Cargo should enforce.

## Existing-authority concentration audit

Issue #10 identifies historical concentration in `element.rs`, but M9 responsibilities
already have coherent owners in style, paint, scene geometry, runtime surface, and
renderer modules. A0 does not authorize a broad split.

New visual/motion values belong in focused modules and element authoring delegates to
those values. If implementation proves uncontainable change-coupling, schedule a
separate behavior-preserving decomposition. File length alone is not evidence.

## Clean cutover

M9 intentionally supersedes bounded M6/M8 proof/limited assumptions where required:

- M8 color-only node background becomes brush-valued background; the old parallel
  color-only authority is removed rather than aliased;
- M6 rectangle-only production shape/stroke breadth expands into the accepted neutral
  shape/path/stroke vocabulary without keeping a second primitive family;
- item-local M6 paint transforms remain item-local, while node presentation transform
  becomes the only node-wide visual-motion geometry authority;
- M6 full-surface damage remains truthful until narrower M9 damage is proved; it is
  not replaced merely for optimization;
- no renderer-owned animation, Vello scene, compatibility motion API, or hidden
  fallback preserves superseded pre-1.0 authority.

Current-truth docs and APIs are reconciled only after accepted implementation, not by
this target-architecture PR.

## Deterministic and real-renderer proof model

Permanent observations live in the M9 conformance matrix. Implementation must prove:

- public/headless deterministic sampling by manually advanced `ManualClock`, never
  sleeps/wall-clock timing as oracle;
- exact intermediate/end, delay/repeat, replacement/cancellation, preference/reduced-
  motion, and cache/invalidation behavior;
- exact paint/hit/focus/semantic correlation under presentation motion;
- layout/text-affecting motion through the accepted M8 authority;
- neutral scene/resource identity independent of backend/device/raster state;
- real-wgpu paths/strokes/gradients/images/nine-slice/groups/shadows and representative
  motion, retained retry, and cache/device re-realization;
- positive, negative, diagnostic, and trace evidence without private expected runtime
  or software expected renderer.

Human visual evidence may supplement structural correctness, but byte-identical
cross-backend GPU output is not the sole oracle where legitimate quantization differs.

## Derived serial implementation sequence

After accepted-main validation of A0, derive child issues in this order:

1. **M9A — production visual/composition vocabulary and real-wgpu realization**:
   style ownership cutover, shapes/paths/strokes/brushes/images/nine-slice/groups/
   shadows, transform/clip/composition ordering, logical bounds/hit, private Lyon
   adoption, cache/re-realization, and renderer realization; no motion yet.
2. **M9B — deterministic transitions and timelines**: runtime lifecycle/sampling,
   exact delay/keyframe/repeat semantics, typed effective property overrides,
   same-property precedence, interpolation/effects, cache compatibility/revisions,
   redraw, preference policy, reduced motion, and presentation geometry.
3. **M9C — integrated production closure**: public manual-time corpus,
   hover/focus/active transitions, layout/text-affecting motion, real-wgpu visual/
   motion evidence, retained retry/re-realization, and final authority cleanup.

Do not create implementation child issues until A0 is owner-accepted, squash-merged,
and accepted-main validated.

## Consequences

- Common production desktop/game visuals are neutral-contract expressible.
- M8 hover/focus/active state can transition without duplicate interaction state.
- Animated presentation geometry stays physically and semantically truthful.
- Motion never mutates authored state, derives behavior from renderer time, or creates
  a parallel layout/cache authority.
- Public APIs are insulated from Lyon/Peniko/Vello/backend churn.
- Renderer complexity grows for paths, gradients, groups and shadows, but remains
  disposable realization state.
- Deferred breadth includes arbitrary shaders/filters, additional blend modes,
  inside/outside strokes, tiled nine-slice, focal/conic gradients, path morphing,
  additive same-property animation layers, generic completion callbacks and visual
  animation editors.

## Validation obligations

The M9 conformance matrix owns permanent observations. Each implementation slice must
start from accepted `main`, revalidate exact dependency patches/features/MSRV/
licenses when first adopting Lyon, run canonical `cargo validate`, pass exact-head
hosted CI, receive complete-diff cold review with zero unresolved debt, and undergo
bounded accepted-main/current-truth reconciliation before rows are promoted.
