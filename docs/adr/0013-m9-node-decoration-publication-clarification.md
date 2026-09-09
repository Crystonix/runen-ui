# ADR 0013: M9 node-decoration publication clarification

> **Category:** ADR
>
> **Status:** Accepted target amendment on exact-head owner acceptance
>
> **Decision date:** 2026-09-09
>
> **Milestone:** M9
>
> **Reviewed baseline:** `d785bf2715502ee391bfe3cf775542e36526b9c9`
>
> **Scope:** narrowly amends ADR 0010 for common node background/outline geometry,
> ownership, and inherited M6 paint ordering. ADR 0011 remains path/ellipse/shadow-
> morphology authority. ADR 0012 remains composition-group ordering/scope authority.

## Context

M9A has converged renderer-neutral shape/stroke/brush/image vocabulary, static
presentation geometry, composition-group structure, individual paint-item bounds,
and recursive group/effect bounds on its in-flight implementation branch. Before
implementing the remaining computed node outline, source-first review found one
observable contract gap.

ADR 0010 assigns common node background-brush and outline policy to computed style /
runtime, and the public `Outline` value already states that runtime selects the node's
authoritative outline geometry. ADR 0012 says node outline is ordinary node paint and
does not itself create isolation. Inherited M6, however, makes paint layer and
contribution ordering observable.

The accepted contracts did not yet define:

- which exact node-local boundary common background and outline use;
- whether common background remains privately manufactured by built-in widgets or is
  normalized into runtime publication together with outline;
- the exact inherited-M6 ordering of those decorations relative to widget-authored
  paint and runtime-shaped text.

Choosing those details only in implementation would silently create architecture.
This ADR freezes the narrow missing contract before implementation resumes.

## Decision

### One authoritative node-decoration shape

Runtime derives one node-decoration shape from the node's **final owner-local layout
box**:

```text
LogicalRect(0, 0, final_local_width, final_local_height)
```

and the effective resolved `ComputedStyle::radius()`.

- absent radius, or four zero radii, uses `SceneShape::Rect`;
- otherwise runtime uses `SceneShape::RoundedRect` with that exact `Radius`;
- RunenUI's existing rounded-rectangle normalization/containment semantics remain the
  geometry authority; no renderer or dependency re-normalizes the corners;
- the same node-decoration shape is the authoritative geometry for the common
  computed-style background and outline;
- the outline is the accepted centered `StrokeStyle` on that exact boundary.

This decoration shape is paint-only. It does not implicitly clip arbitrary widget
content and does not revise physical hit, focus, layout, or semantic geometry.

### Common background and outline publication are runtime-owned

Common computed-style `background` and `outline` are normalized into ordinary
renderer-neutral paint items by `runenui_runtime` during staged paint publication.
They are not separate built-in-widget interpretations.

After the implementation cutover:

- built-in widgets do not privately manufacture the common background;
- there is one production interpretation of common node background/radius/outline;
- custom widget `PaintContribution` continues to own content-specific arbitrary
  fill/stroke/path/image paint;
- a custom widget may intentionally paint equivalent colors/shapes as content, but it
  does not redefine the semantics of the node's common background/outline properties.

This is a clean pre-1.0 authority cutover, not a compatibility duplication.

### Inherited M6 ordering stays authoritative

Runtime-synthesized background and outline are ordinary node-owned paint items. They
use:

- the node's mounted logical preorder;
- `SceneLayer::ZERO`;
- identity item-local transform;
- the same node-presentation transform and final owner placement used by ordinary
  node paint, composed exactly once;
- no invented item-local clip.

For items from the **same node and `SceneLayer::ZERO`**, contribution-local order is:

1. common node background, when present;
2. widget-authored paint items in exact authored relative order;
3. runtime-shaped text runs in their existing deterministic order;
4. common node outline, when present.

This local-order rule does not create an always-back or always-front escape hatch.
Widget items with another `SceneLayer` continue to follow the inherited global M6
ordering key exactly:

```text
(layer, mounted logical preorder, contribution-local order)
```

Therefore a negative-layer widget item may precede the layer-zero background and a
positive-layer widget item may follow the layer-zero outline. No node-decoration
z-index or renderer re-sort is introduced.

### Group ownership remains unchanged

Background and outline participate in an enclosing runtime-derived node-effect group
exactly like other paint owned by that mounted node, so they contribute through the
ordinary item-bound authority to that group's pre-shadow composed child coverage.
Ancestor runtime node-effect groups likewise capture them through mounted-subtree
scope.

They are **not** members of widget-authored explicit owner-local groups. Those groups
remain structurally limited to entries of the exact `PaintContribution` that authored
them, as frozen by ADR 0012 and the accepted M9A explicit-group checkpoint.
Runtime-synthesized decoration therefore follows the same separation already required
for runtime-shaped text: outside explicit widget groups, while still captured by any
enclosing runtime node-effect group.

The outline itself does not create isolation. Node opacity/shadow group existence and
shadow derivation remain controlled by ADR 0012 and ADR 0011 respectively.

### Degenerate and transparent decoration stays ordinary paint semantics

A present background or outline uses the existing generic fill/stroke vocabulary.
Brush/color alpha does not become a second geometry or grouping authority.

A zero-width outline retains M9VIS-03's literal no-coverage behavior and is never a
backend hairline. A renderer may avoid disposable raster work for zero visual
coverage only when doing so does not alter the immutable scene facts or accepted
ordering semantics.

## Consequences

- resolved `Radius` now has one concrete common-decoration paint meaning rather than
  being ignored by the built-in background path;
- common background and outline work for arbitrary mounted elements without asking a
  widget implementation to reinterpret those style properties;
- renderer realization receives only ordinary generic fill/stroke items and needs no
  node-decoration-specific backend contract;
- item/group bound logic requires no parallel decoration geometry model;
- explicit widget groups retain their already-accepted owner-local membership
  boundary;
- M6 layer/order authority is preserved rather than replaced by CSS-like implicit
  background/outline stacking.

## Rejected alternatives

### Keep built-in background paint and synthesize only outline

Rejected because it leaves two ownership paths for common node decoration and keeps
`Radius` dependent on widget-specific paint behavior.

### Give background/outline implicit always-back/always-front ordering

Rejected because that silently overrides inherited M6 `SceneLayer` authority and
creates a second stacking model.

### Let runtime decoration enter widget-authored explicit groups

Rejected because explicit owner-local groups own only structure authored by their
exact `PaintContribution`; admitting runtime decoration would weaken the already
accepted no-foreign-member boundary and diverge from runtime-shaped text.

### Let the renderer infer the decorated node box/radius

Rejected because it would move logical geometry and style interpretation downstream
of the immutable renderer-neutral publication.

### Make outline an isolation/group effect

Rejected because ADR 0012 already freezes outline as ordinary node paint; opacity and
ordinary shadows remain the initial node-effect isolation triggers.

## Conformance and implementation gate

This correction changes no M9 row ID, count, status, or delivery ownership. The M9
conformance matrix is only narrowed to reference this exact decoration geometry and
ordering where relevant.

No production implementation may rely on this ADR until this correction is
explicitly owner-accepted, squash-merged, and accepted-main validated. After that
gate, M9A issue #182 may resume on its existing feature branch by synchronizing the
accepted correction and implementing the clean runtime decoration cutover.
