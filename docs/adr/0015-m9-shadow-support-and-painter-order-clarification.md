# ADR 0015: M9 shadow support and painter-order clarification

> **Category:** ADR
>
> **Status:** Accepted target amendment on exact-head owner acceptance
>
> **Decision date:** 2026-09-11
>
> **Milestone:** M9
>
> **Reviewed baseline:** `6f110e5054ca58eff9abfb3d34f7519fe37bbb72`
>
> **Owner:** #197
>
> **Acceptance:** this amendment becomes accepted only after the exact M9A0R5
> package containing it is explicitly accepted by the repository owner,
> squash-merged, and accepted-main validated. Acceptance clarifies target
> architecture only; it does not claim M9 production implementation.

## Context

ADR 0010 makes runtime publication the sole renderer-neutral composition authority,
ADR 0011 freezes Euclidean signed shadow spread and finite `3 * sigma` blur support,
and ADR 0012 freezes group nesting, contraction, subtree scope, and the structural
effect order `children -> shadows -> group clips -> group opacity -> parent`.

The source-first audit before production composition-group and ordinary-shadow
realization exposed two remaining ambiguities at that boundary.

First, ADR 0011 deliberately did **not** redefine ADR 0010's inherited composed child
coverage as a new source-alpha rule. That was the correct scope decision for M9A0R:
primitive brushes, images, shaped text, opacity, resource sampling, and nested effects
were not yet being implemented as shadow sources. Production nesting now requires the
missing rule. A parent shadow needs one deterministic neutral set from a nested child
group, but renderer target alpha cannot supply that authority:

- image payload alpha would make provider pixels part of framework geometry;
- shaped-text atlas/MSDF alpha would make raster scale and renderer realization part of
  framework geometry;
- ordinary-shadow alpha may vary with the renderer's accepted blur approximation
  inside ADR 0011's finite support; and
- cache/device loss could otherwise change the geometry from which an ancestor shadow
  is derived.

M9VIS-08 already rejects a second shadow-coverage/source-alpha/image-sampling
authority. The missing neutral support rule therefore has to be frozen by RunenUI
rather than inferred from realized RGBA.

Second, accepted authority says ordinary shadow lists are ordered and says that group
shadows occupy the effect phase after composed children, but it does not yet state the
observable source-over painter relationship among multiple colored shadows or between
the shadow layer and the composed child image. Overlapping translucent shadows make
that omission visible. A renderer must not choose CSS, browser, wgpu, or implementation
convention implicitly.

A related clipping problem follows from both ambiguities. A child can lie completely
outside the final surface while its accepted offset/spread/blur support reaches the
visible target. Discarding that child at the final-surface boundary before effect
derivation would therefore be observably wrong.

These are architecture/conformance gaps. They do not justify a renderer-local coverage
model or a backend-selected shadow order.

## Relationship to accepted authority

This ADR narrowly amends ADR 0010, ADR 0011, and ADR 0012 only for:

1. the exact neutral support set used as ordinary-shadow geometry and propagated
   through nested composition groups;
2. the relationship of that set to primitive/resource alpha, item/group opacity,
   group clips, nested effects, and existing conservative `PaintSceneBounds`;
3. the finite neutral support contributed by the ADR 0011 blur cutoff; and
4. exact source-over painter order for an ordered ordinary-shadow list relative to the
   composed child color result.

ADR 0011 remains authoritative for Euclidean signed spread, complete erosion,
accepted offset, and the `3 * sigma` cutoff. ADR 0012 remains authoritative for M6
pre-group order, first-member contraction, mounted visual-subtree scope, group
existence, and recursive group structure. ADR 0013 remains authoritative for runtime
node decoration. ADR 0014 remains authoritative for point-degenerate geometry and
one-item compositing. This amendment does not change those decisions.

The term **neutral effect support** below is a framework geometry product used for
ordinary-shadow derivation and conservative effect realization. It is deliberately
separate from a renderer's sampled RGBA/non-zero-alpha footprint.

## Decision

### Ordinary shadows derive from one neutral effect-support authority

Every paint item and composition group has one renderer-neutral **effect-support set**
in surface logical space.

Effect support answers only whether logical geometry may act as source geometry for an
ordinary shadow or enclosing effect. It does not encode color, fractional alpha,
antialiasing, texture transparency, MSDF coverage, blur weights, or backend samples.
Those remain visual realization/compositing facts.

For one group, let `C` be the union of the neutral effect support of its already-ordered
direct child items and child groups after each child's own accepted clips/effects.
Source-over painter order remains observable for the group's RGBA color result, but it
does not change this set union. `C` is the exact pre-shadow neutral support consumed by
every shadow in that group.

All ordinary shadows in the group derive independently from this same `C`. A shadow
never uses a preceding sibling shadow as its own source. This preserves ADR 0010/0012's
pre-shadow child-coverage authority.

### Primitive effect support is geometry, not sampled alpha

An item first applies its accepted primitive geometry, exact transform chain, and
conjunctive item clips. Its neutral effect support is then defined as follows.

- **Fill:** the accepted logical fill coverage of the `SceneShape` after transform and
  item clips.
- **Stroke:** the accepted centered stroke coverage after transform and item clips,
  including the ADR 0014 degenerate/tangent/topology rules.
- **Image:** the union of the runtime-resolved destination patch rectangles after item
  transform and clips. Ordinary fit may contribute one resolved patch; nine-slice may
  contribute several. Source crop coordinates and payload texels affect sampled color,
  not neutral effect support inside an already-resolved destination patch.
- **Shaped text:** the positioned filled glyph-outline support determined by the exact
  immutable shaped-text resource facts: selected font bytes/face, normalized variation
  and synthetic-style facts, glyph IDs, and logical glyph positions, followed by the
  shaped-run/item transform and item clips. MSDF/atlas resolution and sampled raster
  alpha are not effect-support authority. A renderer that cannot realize an accepted
  glyph representation must retain its structured capability failure rather than
  substitute atlas alpha as framework geometry.

A singular transform or accepted empty primitive/clip yields empty support exactly as
its governing geometry contract requires.

Brush color/gradient alpha, shaped-text foreground alpha, image payload alpha, and item
opacity do **not** erase or shrink this neutral support. They affect the visual source
that is source-over composed, not the geometry supplied to ordinary-shadow support.
This is intentional: otherwise image payload inspection, gradient/raster sampling, or
renderer antialiasing would become a second shadow-geometry authority.

Consequently a visually transparent item may still contribute neutral effect support.
That fact is observable only through an enclosing effect; it does not manufacture
ordinary visible item color by itself.

### Nested groups propagate child plus effect support

Given one group's exact pre-shadow neutral support `C`, each ordinary shadow is derived
independently:

1. apply ADR 0011 Euclidean signed spread to `C`;
2. if erosion collapses the set, the shadow support is empty;
3. apply the accepted finite shadow offset; and
4. apply the finite blur-support operation defined below.

Before group clips, the group's neutral output support is the union of:

- the original pre-shadow child support `C`; and
- the neutral support of every ordinary shadow in the group.

The group's conjunctive clips then intersect that complete child-plus-effects support
in the same structural phase that clips the visual group result. The clipped set is the
neutral support propagated when this group participates as a child of its parent.

Group opacity does not shrink or erase neutral support. It applies only once to the
already-composed visual group result, exactly as ADR 0010/0012 require. Shadow color
alpha likewise affects visual shadow color but does not erase the shadow's neutral
support.

Therefore a visually transparent item, shadow, or nested group may remain part of the
neutral geometry from which an enclosing ordinary shadow is derived. This is a
framework choice, not an optimization hint: effect support is geometric and stable
across renderer/raster/resource realization.

### The `3 * sigma` cutoff has one deterministic neutral support envelope

ADR 0011 requires a truncated Gaussian-style blur whose realized shadow has no support
farther than `3 * sigma` on either logical axis from the spread-adjusted, offset source.
To make nested neutral support independent of a renderer's chosen blur weights, this
amendment freezes the corresponding framework support envelope.

For `sigma >= 0`, let `r = 3 * sigma` and let

```text
Q(r) = [-r, r] x [-r, r]
```

be the closed axis-aligned logical square. If `A` is the spread-adjusted and offset
shadow-source set, the shadow's neutral blur support is

```text
B_sigma(A) = A (+) Q(3 * sigma)
```

where `(+)` is the Minkowski sum. For `sigma = 0`, this is exactly `A`; for empty `A`,
it remains empty.

This square is **not** the ADR 0011 spread kernel. Signed spread remains Euclidean disk
dilation/erosion and therefore rotation-invariant. `Q(3 * sigma)` only freezes the
already axis-bounded blur-support envelope implied by the accepted `3 * sigma` cutoff.
A renderer may choose different deterministic blur weights or sampling inside this
envelope, but it must produce no shadow color outside it and may not use its private
non-zero-alpha footprint as the support propagated to an ancestor group.

### Ordinary shadow painter order is explicit

Visual group composition uses source-over only.

For one group's ordered ordinary-shadow list `[S0, S1, ..., Sn]`:

1. start one transparent shadow-color result;
2. realize every `Si` independently from the same pre-shadow support `C`;
3. source-over the shadow colors in authored list order, so `S0` is painted first and
   each later shadow is visually above earlier overlapping shadows;
4. source-over the already-composed child color result **over** the completed shadow
   result, so all ordinary drop shadows are behind the group children;
5. apply group clips to that complete child-plus-shadow color result;
6. apply group opacity exactly once; and
7. source-over the resulting atomic group into its parent at the ADR 0012 anchor.

This painter rule is RunenUI-owned. It is not inherited from CSS, a browser, a graphics
API, or a renderer library. Reordering the authored shadow list may therefore change
visual output where shadows overlap, while every shadow's geometry still derives from
the same `C`.

### Conservative bounds remain metadata, not exact support authority

Existing framework `PaintSceneBounds` remain deterministic conservative AABBs derived
from neutral scene facts. They are not replaced by raster pixels and they are not the
exact effect-support set.

The existing conservative rules remain valid:

- positive spread expands the conservative source AABB by `s` on every side;
- zero spread preserves it;
- negative spread may conservatively retain the pre-shadow AABB when no tighter neutral
  erosion bound is available;
- offset translates it; and
- blur expands every side by `3 * sigma`.

The resulting AABB must contain the exact neutral support defined by this amendment.
An implementation may maintain a tighter disposable realization region, but it cannot
promote that private region to framework authority or publish a bound smaller than the
accepted support.

`Unbounded` remains a conservative top state. A renderer that requires finite
intermediate allocation must fail with structured capability/realization diagnostics
when accepted neutral facts do not permit a safe finite target; it must not guess a
crop from currently realized alpha.

### Final-surface clipping cannot erase an effect source too early

A disposable renderer may crop intermediate targets only when the crop is proven from
accepted neutral support/effect facts to preserve every pixel that can contribute to
the requested final target through the remaining group/effect chain.

In particular, it is invalid to intersect child support with the final surface before
ordinary-shadow derivation merely because the child color itself is off-surface. An
off-surface child may have offset/spread/blur support that reaches the visible surface,
and a nested off-surface effect may similarly contribute to an ancestor shadow.

A renderer may conservatively allocate from framework group/effect bounds, derive a
tighter private region from the same neutral facts, or use another disposable strategy.
Final-surface extent, render-target allocation, raster scale, stencil contents, and
cached alpha never redefine the framework support set.

## Conformance impact

This amendment adds no M9 row and changes no row status.

It sharpens `M9VIS-08` to prove:

- one neutral effect-support authority for fill, stroke, resolved image patches, and
  retained shaped-text glyph outlines;
- item/group opacity, brush/foreground alpha, image payload alpha, shadow color alpha,
  antialiasing, MSDF/atlas samples, blur weights, raster scale, and cache/device state do
  not redefine that support;
- direct child support unions into one pre-shadow `C` and every sibling shadow derives
  independently from that same set;
- exact Euclidean signed spread remains unchanged, including complete erosion;
- the deterministic `Q(3 * sigma)` blur-support envelope is propagated through nested
  groups without turning it into the spread kernel;
- group output support is child plus shadow support, then conjunctive group clips, with
  group opacity excluded from support geometry;
- nested groups use the clipped neutral output support of their descendants rather than
  realized target alpha;
- authored shadow order is observable: first authored paints first, later shadows paint
  above earlier shadows, and the composed child color result paints above all ordinary
  shadows;
- off-surface sources/effects are retained when their accepted downstream support can
  reach the final surface or an ancestor effect; and
- `PaintSceneBounds` remains conservative metadata enclosing exact neutral support,
  never a replacement raster/source-alpha authority.

Negative proof must reject payload-alpha-derived shadow masks as framework geometry,
MSDF/atlas-alpha-derived text support, blur-alpha propagation into ancestor geometry,
per-renderer shadow ordering, shadows painted over child content, sibling-shadow
chaining, final-surface clipping before effect derivation, and target/cache state as a
coverage authority.

The matrix remains exactly 25 rows, all blocked until normal M9 implementation,
proof, owner acceptance, merge, accepted-main validation, and later reconciliation.

## Consequences

- Production group/shadow realization needs a disposable neutral-support realization
  parallel to visual RGBA composition; it cannot derive ancestor shadow geometry by
  thresholding or sampling an intermediate color target.
- The renderer may rasterize the accepted neutral support into private masks for
  morphology/blur, but those masks are reconstructions of framework geometry, not
  retained scene authority.
- Image transparency does not punch holes in ordinary-shadow source geometry; resolved
  destination patches remain the neutral image support contract.
- Shaped-text shadow geometry follows the accepted retained glyph outlines rather than
  MSDF/atlas alpha or pixel resolution.
- Transparent paint/effect values can remain neutral shadow sources. Visual transparency
  and effect-support geometry are intentionally distinct products.
- Nested inner shadows contribute their accepted finite support to an enclosing group's
  support after inner group clips, so outer shadows remain deterministic without
  renderer-alpha feedback.
- Existing conservative effect bounds remain useful for allocation/damage proof, while
  exact morphology and support semantics remain separate.
- Renderer implementations may choose full-surface, conservative-bounds, tiled, or
  other disposable intermediate strategies as long as early clipping cannot remove an
  accepted effect source.

## Rejected alternatives

### Use realized source alpha as shadow geometry

Rejected because image pixels, gradient/raster samples, MSDF/atlas resolution, blur
weights, antialiasing, target format, and device state would become framework geometry
authority. Nested shadows could then change across renderer implementations or cache
reconstruction.

### Treat image payload alpha as neutral image support

Rejected for initial M9 because the accepted image descriptor resolves geometry without
payload-pixel inspection. Making decoded texel alpha part of framework effect geometry
would create a new resource-sampling authority and would make runtime-neutral proof
depend on provider pixels.

### Use conservative AABBs as the shadow source set

Rejected because bounds are intentionally conservative. Morphological spread over an
AABB is not equivalent to spread over the accepted shape, path, glyph, or nested-group
support and would violate ADR 0011.

### Let each renderer choose shadow-list painter order

Rejected because overlapping translucent shadow colors make the order observable. The
ordered public list requires one framework-owned painter interpretation.

### Paint ordinary shadows above child content

Rejected because an ordinary drop shadow is the group's behind-content effect in the
initial M9 vocabulary. Effects that intentionally overlay child content require a
separate explicit neutral effect contract.

### Chain each shadow from the previous shadow result

Rejected because ADR 0010/0012 already requires every group shadow to derive from the
same pre-shadow composed child coverage. Shadow list order affects color composition,
not the geometry source of later sibling shadows.

### Clip every group intermediate to the final surface first

Rejected because offset/spread/blur can move accepted support from an off-surface
source into the final target and because nested effects may feed ancestor effects.

### Make transparent values erase neutral support

Rejected because doing so would require one alpha-support definition spanning solid and
gradient brushes, provider-owned image pixels, shaped-text rasterization, blur kernels,
and antialiasing. Initial M9 instead keeps ordinary-shadow geometry neutral and stable;
alpha remains visual compositing data.

## Non-goals

This amendment adds no production Rust, renderer pipeline, resource payload format,
new public visual primitive, blend mode, filter graph, shadow family, conic gradient,
partial-damage optimization, motion/timeline behavior, new M9 row, or row promotion.
It does not change M6/M7/M8 resource identity, hit/semantic geometry, or retained scene
authority. It only freezes the missing neutral support and ordinary-shadow painter
semantics required before the existing M9A production group/shadow checkpoint may
resume.
