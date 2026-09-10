# ADR 0014: M9 degenerate stroke geometry and primitive compositing clarification

> **Category:** ADR
>
> **Status:** Accepted target amendment on exact-head owner acceptance
>
> **Decision date:** 2026-09-10
>
> **Milestone:** M9
>
> **Reviewed baseline:** `eb67210d69d97dba7885b5a2057b7705653515c2`
>
> **Owner:** #195
>
> **Acceptance:** this amendment becomes accepted only after the exact M9A0R4
> package containing it is explicitly accepted by the repository owner,
> squash-merged, and accepted-main validated. Acceptance clarifies target
> architecture only; it does not claim M9 production implementation.

## Context

ADR 0010 keeps stroke geometry and paint-item composition RunenUI-owned rather than
letting a renderer or geometry library choose public semantics. ADR 0011 additionally
freezes open-path fill closure, authored stroke closure, and ellipse degeneracy.

The source-first audit before production generic fill/stroke realization exposed two
remaining defects at that ownership boundary:

1. accepted M6 ADR 0007 and conformance `PAINT-03` explicitly state that a stroked
   logical rectangle whose own width or height is zero has no coverage, while the
   in-flight M9 item-bound implementation currently reports finite positive-width
   coverage for that case and its private Lyon adapter can hand the degenerate
   rectangle to dependency tessellation; and
2. ADR 0010 says degenerate caps and joins follow neutral geometry rather than backend
   defaults, while ADR 0011 defines structural segment-bearing contours without yet
   freezing how geometrically point-degenerate segments affect fill boundary/winding
   or stroke cap/join topology and how zero-derivative Bézier endpoints choose the
   limiting tangent required by stroke realization.

The same renderer audit also confirmed that dependency tessellation is a decomposition
of one paint item, not additional scene content. A tessellator may emit overlapping
triangles for one logical stroke. If those triangles were independently source-over
blended, one paint item's brush/opacity could be applied multiple times at one sample,
contradicting the inherited item-composition contract.

These are architecture/conformance gaps. Exact-head implementation CI cannot silently
amend the accepted M6 rule or delegate the missing M9 behavior to Lyon.

## Relationship to accepted authority

This ADR narrowly amends ADR 0010 and ADR 0011 only for:

1. zero-extent `Rect` / `RoundedRect` stroke coverage;
2. the geometric contribution of point-degenerate path segments plus limiting endpoint
   tangents used by stroke caps and joins; and
3. the relationship between one logical paint item's coverage and disposable renderer
   tessellation overlap.

ADR 0011 continues to control every other path fill-only synthetic-closure, authored
stroke-closure, malformed-contour-ordering, path-boundary, and ellipse-degenerate rule;
this amendment only fills its missing point-degenerate-segment geometry case. ADR 0012
continues to control composition-group ordering and scope. ADR 0013 continues to
control common node decoration geometry/publication. Inherited M6
source-over/item-order/color/opacity semantics remain authoritative.

## Decision

### Zero-extent rectangle-family shapes do not acquire stroke coverage

The accepted M6 rectangle rule is preserved exactly:

- a `Rect` whose logical width or height is zero has empty fill coverage;
- a `Rect` whose logical width or height is zero also has empty stroke coverage for
  every positive stroke width and every cap/join choice; and
- stroke width zero remains empty regardless of shape.

M9 `RoundedRect` extends the same outer-rectangle geometry family. If its outer logical
rectangle has zero width or height, both fill and stroke coverage are empty. Corner
radii do not manufacture a boundary for an empty outer rectangle. A dependency or
renderer must not reinterpret such a shape as a line, capsule, point, or other
positive-area stroke.

ADR 0011's accepted ellipse rule is unchanged: a zero-width or zero-height ellipse has
no analytic boundary/coverage for M9 realization and remains empty rather than
acquiring backend-defined degenerate stroke geometry.

### Point-degenerate path segments remain structural but contribute no geometry

A validated `ScenePath` remains structural authored content. A line, quadratic, or
cubic verb continues to make its contour segment-bearing for ADR 0011 validation even
when that authored segment is geometrically point-degenerate.

For this contract, one authored segment is **point-degenerate** exactly when its whole
parametric image is one point:

- line: `start == end`;
- quadratic: `start == control == end`;
- cubic: `start == control1 == control2 == end`.

A point-degenerate segment:

- remains present in structural path identity/equality and validation order;
- contributes no fill boundary edge or winding change;
- contributes no stroke centerline coverage;
- supplies no cap or join tangent; and
- is skipped only by geometric evaluation/realization, never deleted from authored
  `ScenePath`.

A contour containing authored segments but no non-point-degenerate segment therefore
has empty geometric fill and stroke coverage. This does not turn it back into a
move-only structural contour: an authored `close` remains valid under ADR 0011 because
validation is structural. An open such contour's synthetic final-to-first fill edge is
also point-degenerate and contributes no fill boundary or winding.

### Stroke topology skips point-degenerate segments without changing contour closure

For stroke geometry, evaluate each contour in authored order while ignoring
point-degenerate segments as centerline/tangent contributors.

For an authored-open contour:

- if no non-point-degenerate segment remains, stroke coverage is empty regardless of
  cap or join style;
- otherwise the start cap belongs to the start of the first non-point-degenerate
  segment and the end cap belongs to the end of the last non-point-degenerate segment;
- point-degenerate segments between non-point-degenerate neighbors do not manufacture
  extra caps or joins; the neighboring non-point-degenerate centerline pieces meet at
  the same authored point under the ordinary join rule.

For an authored-closed contour:

- closure remains structural even when an individual authored closing edge is
  point-degenerate;
- if no non-point-degenerate segment remains, stroke coverage is empty;
- otherwise the contour has a closed seam and therefore no endpoint caps; the last and
  first non-point-degenerate centerline pieces participate in the seam join;
- a non-point-degenerate authored final-to-first closing edge remains ordinary
  centerline coverage exactly as ADR 0011 requires.

The synthetic final-to-first edge used only for open-contour fill remains excluded from
stroke geometry.

### Bézier cap/join tangents use the exact limiting polynomial direction

Dependency fallback tangent rules are not public semantics.

For a non-point-degenerate segment, the endpoint tangent direction is the first
non-zero endpoint chord in polynomial control order. Equality/non-zero tests operate
on the exact validated logical points; normalization of the resulting direction is
realization math and does not change path identity.

At the segment start:

- line: `end - start`;
- quadratic: first non-zero of `control - start`, then `end - start`;
- cubic: first non-zero of `control1 - start`, `control2 - start`, then `end - start`.

At the segment end, the direction is the direction of travel into the endpoint:

- line: `end - start`;
- quadratic: first non-zero of `end - control`, then `end - start`;
- cubic: first non-zero of `end - control2`, `end - control1`, then `end - start`.

These are the limiting tangent directions of the accepted polynomial segments when
adjacent control points coincide. If every candidate vector is zero, the segment is
point-degenerate under the rule above.

Caps use the applicable first/last non-point-degenerate endpoint tangent. Joins use the
incoming and outgoing limiting tangents of neighboring non-point-degenerate segments.
Ordinary butt/round/square and miter/bevel/round geometry remains the ADR 0010 stroke
contract.

For a miter join, if no finite miter satisfying the accepted geometry exists, or the
required finite miter ratio exceeds the validated limit, the result falls back to
bevel. A dependency-specific cusp/miter recovery mode does not become RunenUI
semantics.

### One paint item is one compositing source

A `PaintSceneItem` contributes one logical primitive coverage set after its item clips.
Renderer tessellation, triangle subdivision, curve flattening, stencil fragments,
MSAA samples, or other disposable realization pieces are not additional scene items
and do not independently participate in scene-order source-over composition.

For one fill or stroke paint item at one target sample:

1. determine whether the sample is covered by that logical primitive after all item
   clips under the accepted geometry contract;
2. if covered, evaluate the item's brush and multiply source alpha by the item's
   validated item opacity exactly once; and
3. source-over that one resulting source exactly once into the containing composition
   result at that item's accepted scene position.

Multiple renderer triangles/fragments representing the same primitive sample therefore
must not multiply brush alpha/item opacity or perform repeated source-over for that
single item. A renderer may use stencil, a temporary coverage target, analytic
coverage, or another disposable equivalent technique, but overlap multiplicity of its
private decomposition is not observable RunenUI composition.

This rule does not require binary raster edges or prohibit antialiasing/MSAA.
Raster-edge sampling remains renderer realization quality. It only prohibits treating
private decomposition overlap as repeated logical paint-item composition.

Group opacity remains governed separately by ADR 0010/0012 and still applies once to
the already-composed group result.

## Conformance impact

This amendment adds no M9 row and changes no row status.

It sharpens `M9VIS-02` to prove that point-degenerate authored segments remain
structural/validation facts but create no fill boundary, winding change, or geometric
coverage by themselves.

It sharpens `M9VIS-03` to prove:

- inherited zero-extent rectangle stroke emptiness;
- zero-extent rounded-rectangle stroke emptiness;
- point-degenerate path-segment elision without structural deletion;
- deterministic cap/join topology after such elision;
- limiting quadratic/cubic endpoint tangent selection; and
- one logical stroke primitive cannot acquire repeated source-over merely because
  renderer tessellation overlaps itself.

`M9VIS-10` continues to own the dependency boundary and must negatively prove that
Lyon's tessellation/default/recovery behavior does not become fill/stroke geometry or
composition authority.

The matrix remains exactly 25 rows, all blocked until normal M9 implementation,
proof, owner acceptance, merge, and accepted-main validation occur.

## Consequences

- The in-flight M9 item-bound regression for zero-extent `Rect` stroke must be repaired
  before the current M9A renderer work can converge.
- The private Lyon adapter must reject empty rectangle-family strokes and must not let
  point-degenerate path segments manufacture dependency-specific fill edges,
  caps, joins, or tangents.
- Production generic stroke realization must treat tessellation as one primitive's
  coverage decomposition. Naive alpha blending of every overlapping Lyon triangle is
  insufficient proof for translucent/self-overlapping strokes.
- The renderer remains free to choose disposable coverage machinery as long as it
  preserves the accepted one-item composition result and logical geometry authority.

## Rejected alternatives

### Keep the M9 zero-extent rectangle bounds behavior

Rejected because it directly contradicts accepted M6 ADR 0007 / `PAINT-03`, which M9
never amended.

### Let Lyon decide zero-length segments, endpoint tangents, or cusp recovery

Rejected because ADR 0010 already makes degenerate cap/join geometry RunenUI-owned and
ADR 0011 makes path fill boundary topology RunenUI-owned. Changing Lyon version or
tessellation strategy cannot change public fill/stroke coverage.

### Treat every authored zero-length segment as an endpoint-cap primitive

Rejected because square-cap orientation and join topology would require an arbitrary
fallback direction. Point-degenerate segments instead remain structural but contribute
no centerline/tangent geometry.

### Blend every tessellated triangle as a separate source

Rejected because triangle decomposition is renderer machinery, not scene order.
Overlapping triangles from one logical primitive must not multiply that item's source
alpha/opacity/source-over operation.

## Non-goals

This amendment adds no production Rust, renderer code, Cargo dependency, gradient,
composition-group render target, shadow kernel/morphology, partial damage, motion,
new M9 row, row promotion, or renderer cache/public API. It does not choose the exact
wgpu stencil/offscreen implementation used to enforce one-primitive coverage; that
remains subordinate renderer realization.