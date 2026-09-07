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

Initial shapes are:

- rectangle;
- normalized rounded rectangle;
- ellipse;
- immutable validated path using move, line, quadratic, cubic, and close verbs.

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

Initial brushes are:

- solid `Color`;
- linear gradient;
- radial gradient.

Conic gradients and arbitrary shader brushes are deferred. Gradient geometry is
logical. Stops have finite offsets in `[0, 1]`, stable nondecreasing order, at least
two entries, and may share offsets for hard stops.

Existing `Color` remains straight-alpha sRGB8 public storage. Gradient evaluation and
continuous color animation use premultiplied linear-sRGB interpolation with
deterministic conversion, clamping, and rounding back to the public representation.

Item opacity is validated item-local alpha. Group opacity is validated alpha applied
once to the already-composed group result. **Source-over is the only initial group
composition mode.** Additional blend modes require a later explicit neutral contract;
backend blend/shader enums never become widget semantics.

### Images resolve fit before the renderer

`ResourceRef` remains complete opaque image identity. M9 adds a neutral image
descriptor pairing an image `ResourceRef` with immutable intrinsic pixel dimensions
supplied by the logical resource owner. Intrinsic dimensions are descriptive logical
metadata, never a second identity/cache handle.

Image paint owns:

- normalized source crop;
- logical destination rectangle;
- fit mode: fill, contain, cover, none, or scale-down;
- normalized two-axis alignment;
- optional nine-slice source insets plus explicit logical destination edge widths.

Fit is resolved before renderer realization. Missing/invalid intrinsic metadata
diagnoses; the renderer never guesses another fit. Crop, fit, alignment, nine-slice,
layout, raster scale, atlas placement and device state do not remint `ResourceRef`.

Nine-slice source insets cannot overlap after normalization. If destination size is
smaller than opposing logical edge widths, opposing edges are proportionally scaled
so center extent is never negative. The center may collapse to zero but never flips.

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

A group may carry:

- parent snapshot-local group reference;
- ordered conjunctive clips;
- validated group opacity;
- ordered ordinary drop shadows accepted by this ADR.

Paint items reference their containing group. Group IDs are snapshot-local
structure/order only. They have no mounted identity, lifecycle, reconciliation key,
semantic identity, widget state, or cross-publication authority. Runtime derives the
table while staging a publication. Renderer offscreen targets are disposable
realization state.

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
sampled once and composed consistently into:

- paint geometry;
- physical hit geometry;
- directional-focus geometry;
- semantic bounds;
- presentation-relative clip geometry.

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

A motion target is an explicit RunenUI-owned property identity. Initial targets are:

- supported computed-style visual properties;
- node presentation transform/opacity;
- explicitly accepted numeric layout fields whose endpoints share one domain, such as
  length-to-length or percent-to-percent dimensions/offsets/gaps;
- M8 layout-affecting style values such as padding.

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
   target change. They carry stable authored animation ID, duration, delay, ordered
   normalized keyframes, easing, and repeat policy: once, finite count, or forever.
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

Reconciliation/lifecycle rules:

- unchanged explicit animation ID/spec preserves progress;
- changed spec under the same ID cancels/replaces at current runtime time;
- removal cancels;
- mounted replacement/removal/shutdown cancels exact mounted generation;
- style target replacement starts from current sampled presented value;
- zero-duration motion reaches terminal sample in the next staged publication with
  no fake one-frame animation;
- finite timelines complete after their final iteration;
- forever timelines never complete naturally.

Completion/cancellation cleanup commits with runtime-owned state/publication planning.
Renderer success/failure is never the clock or completion trigger.

### Easing/interpolation remains RunenUI-owned

Initial easing is linear plus validated cubic-Bezier timing curves and named
conveniences defined in terms of those curves. Input/output are normalized `[0, 1]`
scalars with exact endpoints. Easing owns no clock or timeline state.

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

For one candidate, the sampled effective value is part of compatibility for only the
stages that consume that property. Layout/text caches remain reusable when their exact
effective inputs are unchanged; paint/hit/focus/semantic caches remain reusable when
their exact relevant effective facts are unchanged. A later clock instant that maps
to the same effective value does not force unrelated recomputation merely because
time advanced.

Existing publication/product revisions advance only when the corresponding immutable
published product content changes under their accepted rules. Active motion may still
keep future redraw requested even when two consecutive samples quantize to equal
published content. Redraw demand is scheduling state, not scene/resource identity.

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

| Candidate | Observed upstream state | Cost / authority fit | M9 decision |
|---|---|---|---|
| Lyon | Actively maintained; meta crate `1.0.19`; current upstream subcrates include `lyon_path 1.0.19`, `lyon_algorithms 1.0.21`, `lyon_tessellation 1.0.22`; MIT OR Apache-2.0; manifests inspected do not declare `rust-version`; path/tessellation defaults use `std`, with small geometry/numeric dependencies and optional serde; no wgpu dependency | Purpose-built path algorithms/tessellation and no retained UI/renderer scene requirement; exact patch MSRV still must be proven | **Adopt privately, by exact required subcrates rather than the meta crate by default.** Public RunenUI path/stroke types remain authoritative. First dependency PR proves exact patches, features, transitives, licenses, Rust 1.93, and benchmarks/corpus where relevant. |
| Kurbo | `0.13.1`; MSRV `1.85`; Apache-2.0 OR MIT; actively maintained; default `std`; curve library with `f64` canonical geometry and small generic dependencies; no wgpu dependency | Strong geometry algorithms, but using it beside Lyon would create a second private geometry model/conversion path without removing tessellation | **Do not adopt for M9 initially.** Reconsider only for a concrete algorithm Lyon cannot supply cleanly. |
| Peniko | `0.6.1`; MSRV `1.85`; Apache-2.0 OR MIT; actively maintained; default `std`; depends on Kurbo, color, smallvec and resource-handle vocabulary; no wgpu dependency | Brush/gradient/image/blend/resource types overlap the exact public authority M9 must freeze | **Reject as public or parallel paint vocabulary.** Do not add merely to avoid RunenUI-owned adapters/types. |
| Vello | `0.10.0`; MSRV `1.89`; Apache-2.0 OR MIT; actively maintained; own renderer/scene stack; current workspace uses wgpu/naga `29.0.3` and Peniko/Kurbo | Broad renderer/build/dependency footprint; adopting it wholesale creates a second scene/renderer authority and a second wgpu major beside RunenUI wgpu 30 | **Reject wholesale for M9.** A future isolated algorithm may be reconsidered only if it transfers no retained scene/timeline/renderer authority. |
| `keyframe` | Current published `1.1.1` is old relative to this milestone and includes animation-sequence/time semantics | Small but would import a second sequence abstraction while still not encoding RunenUI property compatibility, invalidation, mounted lifetime, preferences, or publication | **Build bounded easing/timeline semantics in RunenUI.** |
| `interpolation` / `enterpolation` and similar generic helpers | Generic interpolation/easing families; `enterpolation` is substantially broader than M9's initial linear/cubic-Bezier need | Generic `Lerp`-style adoption encourages exactly the inferred animatability M9 rejects and does not solve runtime authority/invalidation | **Do not adopt initially.** Implement the bounded property-specific interpolation layer directly. |

The selected Lyon boundary is subordinate algorithms/data realization only. Lyon never
owns public path identity, mounted state, scene order, hit target identity, style,
animation, resource identity, cache compatibility, or renderer lifetime.

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
- exact intermediate/end, replacement/cancellation, preference/reduced-motion, and
  cache/invalidation behavior;
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
   typed effective property overrides, same-property precedence, interpolation/
   effects, cache compatibility/revisions, redraw, preference policy, reduced motion,
   and presentation geometry.
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
  inside/outside strokes, path morphing, conic gradients, additive same-property
  animation layers, generic completion callbacks and visual animation editors.

## Validation obligations

The M9 conformance matrix owns permanent observations. Each implementation slice must
start from accepted `main`, revalidate exact dependency patches/features/MSRV/
licenses when first adopting Lyon, run canonical `cargo validate`, pass exact-head
hosted CI, receive complete-diff cold review with zero unresolved debt, and undergo
bounded accepted-main/current-truth reconciliation before rows are promoted.
