# ADR 0011: M9 path-fill and shadow-spread clarification

> **Category:** ADR
>
> **Status:** Accepted target amendment on exact-head owner acceptance
>
> **Decision date:** 2026-09-07
>
> **Milestone:** M9
>
> **Reviewed baseline:** `99f83cca65da88649243637f3693e5ca978e7612`
>
> **Acceptance:** this amendment becomes accepted only after the exact M9A0R
> package containing it is explicitly accepted by the repository owner,
> squash-merged, and accepted-main validated. Acceptance clarifies target
> architecture only; it does not claim M9 production implementation.

## Context

ADR 0010 deliberately keeps public shape/path coverage and ordinary shadow geometry
RunenUI-owned rather than delegating those semantics to a renderer or geometry
library. The first M9A implementation audit exposed two places where that ownership
was asserted but the actual rule was still under-specified:

- open path contours had no frozen fill-closure rule distinct from explicit stroke
  closure; and
- signed shadow spread had no frozen geometric operation over arbitrary composed
  coverage.

Those omissions cannot be resolved by adopting Lyon, SVG, CSS, wgpu, or a raster
kernel default. In particular, Lyon's general path hit-test accepts a flattening
tolerance, which makes it unsuitable as the public logical containment oracle.

## Relationship to ADR 0010

This ADR **narrowly amends ADR 0010** only for:

1. path contour fill closure and explicit stroke closure;
2. path/ellipse boundary and degenerate containment behavior; and
3. ordinary drop-shadow spread and its logical effect-bound derivation.

For those subjects, ADR 0011 controls. Every other ADR 0010 decision remains
unchanged and authoritative. This is an amendment relationship, not a second M9
visual architecture.

## Decision

### Path contours distinguish fill closure from authored stroke closure

Each `move` starts one contour. A later `move` ends the preceding contour without
turning it into an explicitly closed stroke contour.

A contour is **segment-bearing** only after at least one line, quadratic, or cubic
segment follows its starting `move`. A move-only contour has no fill or stroke
coverage. `close` on a move-only contour rejects.

For **fill coverage**, every segment-bearing contour is logically closed. If the
contour has no authored `close`, fill evaluation behaves as if one straight segment
from the contour's final point to its first point were present. That synthetic edge
exists only for fill topology and fill-boundary membership.

For **stroke coverage**, only authored segments exist. An explicit `close` adds the
real final-to-first centerline segment and creates a closed seam whose corner uses the
authored join semantics. An open contour retains two endpoints using the authored cap
semantics. The synthetic fill-closing edge of an open contour is never stroked.

`close` is valid only for the current segment-bearing open contour. A second `close`,
or a line/quadratic/cubic segment after `close` without a new `move`, rejects rather
than inheriting a dependency builder's recovery behavior.

### Path fill containment is tolerance-independent framework behavior

A point exactly on an authored path segment or curve is inside path fill coverage. A
point exactly on the synthetic fill-closing edge of an open segment-bearing contour is
also inside fill coverage.

For points not on a boundary, the authored `NonZero` or `EvenOdd` fill rule is applied
to the logically closed fill contours.

Logical containment and logical bounds remain RunenUI facts. Renderer tessellation,
raster scale, cache state, and a dependency flattening tolerance cannot change the
result. An implementation may use private helpers or analytic/numeric subroutines,
but their tolerances are implementation machinery and cannot become observable public
semantics.

### Ellipse containment is analytic and explicitly degenerate

An ellipse is an analytic closed shape over its logical bounding rectangle.

- zero logical width or zero logical height has empty fill and hit coverage;
- positive width and height use the analytic ellipse;
- points on the analytic ellipse boundary are inside ellipse coverage.

This does not revise the inherited M6 half-open rectangle and rounded-rectangle
containment contract. Equivalent geometry expressed through different accepted shape
variants is therefore not required to share measure-zero boundary membership where
their explicit boundary contracts differ.

### Shadow spread is Euclidean morphological spread

Let `C` be the exact pre-shadow **source-alpha support** after the shadow owner's
children/items have been clipped and source-over composed, but before shadow spread,
offset, and blur. A logical point belongs to `C` exactly when the resulting composed
source alpha at that point is non-zero. Fully transparent source contributes no
shadow support merely because its primitive geometry has area.

For resource-backed content, the immutable payload bound to the complete accepted
`ResourceRef` supplies its source alpha during realization. That does not transfer
shadow semantics to the renderer: the resource binding, source-over order, clip
semantics, and non-zero-alpha support rule are neutral accepted facts. Runtime may use
conservative neutral primitive/resource bounds for framework effect bounds without
inspecting resource pixels.

For finite signed spread `s`:

- `s > 0`: spread is the Minkowski sum of `C` with the closed Euclidean disk of
  radius `s`;
- `s = 0`: spread preserves `C` exactly;
- `s < 0`: spread is Euclidean erosion by the closed disk of radius `|s|`; a point
  remains exactly when that entire disk centered at the point lies inside `C`.

Complete erosion produces empty shadow coverage.

The disk operation is intentionally rotation-invariant. Rotating equivalent
pre-shadow coverage cannot change the meaning of spread merely because logical axes
changed. An axis-aligned square/kernel spread is not an accepted substitute.

After spread, the accepted shadow offset translates the spread-adjusted coverage.
The ADR 0010 truncated Gaussian-style blur then has support no farther than
`3 * sigma` on either logical axis from that translated coverage.

Renderer realization may approximate alpha distribution inside the accepted finite
support as required by ordinary rasterization, but it may not substitute another
spread geometry, extend support beyond the accepted cutoff, or derive framework
logical bounds from raster pixels.

### Framework shadow bounds are deterministic and conservative

Framework effect/damage bounds describe accepted logical coverage and need not be the
smallest mathematically possible AABB. They may conservatively bound source-alpha
support from neutral primitive/resource geometry; they do not need payload-alpha
inspection to remain authoritative.

Given conservative pre-shadow support bounds:

1. positive spread expands each side by `s`;
2. zero spread preserves those bounds;
3. negative spread may conservatively retain the pre-shadow bounds when a tighter
   erosion bound is unavailable, but an actually empty eroded support set remains
   empty;
4. apply the finite logical shadow offset; and
5. expand each axis by `3 * sigma` for blur support.

A renderer may keep tighter private realization bounds. It must never expose or rely
on a framework bound smaller than actual accepted logical shadow coverage.

## Conformance impact

This amendment changes no M9 row count or status. It sharpens existing obligations:

- `M9VIS-02` proves open-contour fill closure, boundary inclusion, move-only and
  ellipse degeneracy, and tolerance-independent logical containment;
- `M9VIS-03` proves explicit-close seam/join behavior versus open-contour endpoint
  cap behavior; and
- `M9VIS-08` proves source-alpha-support ownership, Euclidean disk
  dilation/erosion, rotation invariance, conservative logical effect bounds,
  complete-erosion emptiness, and finite `3 * sigma` support.

All 25 M9 rows remain `blocked` until their implementation and proof are accepted
through the normal serial delivery/reconciliation process.

## Consequences

- `runenui_core` cannot make Lyon's tolerance-based hit-test the public path
  containment oracle.
- Renderer tessellation and shadow-mask kernels remain disposable realization, not
  geometry authority.
- Path fill and stroke may share authored segments while deliberately differing at
  an unclosed contour's final-to-first edge.
- Fully transparent composed source does not cast a shadow solely because its neutral
  primitive bounds are non-empty.
- Shadow spread has one rotation-invariant meaning across paths, ellipses, images,
  text-derived/group coverage, raster scales, and renderer devices.

## Rejected alternatives

### Let Lyon/SVG/backend defaults decide open-fill behavior

Rejected because it silently transfers public geometry semantics to an implementation
choice and can diverge between logical hit/bounds and rendered coverage.

### Use Lyon's tolerance-based path hit-test as the logical oracle

Rejected because changing a flattening tolerance could change public containment.
Such tolerance may remain a private approximation tool only where conformance proves
it cannot alter accepted logical results.

### Use axis-aligned square spread

Rejected because square dilation/erosion is orientation-dependent. Ordinary spread
must not change meaning merely because equivalent source coverage is rotated relative
to logical axes.

### Use primitive AABBs as shadow source coverage

Rejected because transparent or clipped-away source could then cast a shadow. AABBs
are permitted only as conservative framework effect-bound inputs, not as the source
alpha-support definition.

### Let raster morphology define spread

Rejected because device scale, kernel resolution, or cache strategy would then become
framework geometry authority.

## Non-goals

This amendment adds no production Rust, dependency, renderer, workflow, motion,
filter family, blend mode, path morphing, new M9 row, or new implementation slice.
