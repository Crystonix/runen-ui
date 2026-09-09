# ADR 0012: M9 composition-group ordering clarification

> **Category:** ADR
>
> **Status:** Accepted target amendment on exact-head owner acceptance
>
> **Decision date:** 2026-09-09
>
> **Milestone:** M9
>
> **Reviewed baseline:** `a4ea57a3a1aec21622ec49a3799d5cc415cc4b5c`
>
> **Acceptance:** this amendment becomes accepted only after the exact M9A0R2
> package containing it is explicitly accepted by the repository owner,
> squash-merged, and accepted-main validated. Acceptance clarifies target
> architecture only; it does not claim M9 production implementation.

## Context

ADR 0010 requires two properties that are individually precise but jointly
under-specified once composition groups are introduced:

- inherited M6 `PAINT-05` orders every paint item globally by stable ascending
  `(layer, mounted logical preorder, contribution-local order)`; and
- M9 composition groups are atomic source-over units whose children compose before
  group effects, clips, and one group-opacity application, after which the complete
  group result composes into its parent.

If two items in one group have an outside item between them in the inherited M6 order,
an atomic group cannot preserve that interleaving. For example, M6 may require
`A1, B, A2` while `A1` and `A2` belong to group `G`. Once `G` is isolated, the parent
can contain `G, B` or `B, G`, but cannot contain `A1, B, A2` without dissolving the
group. Per-child opacity is not an equivalent escape because overlapping children
must receive group opacity once after their composed result.

ADR 0010 says runtime owns exact scene ordering and snapshot-local group
nesting/order, but it does not freeze the parent insertion point for such a group or
the exact mounted scope of node opacity/shadows. Leaving either choice to runtime or
renderer implementation would create an accidental second scene-order authority.

## Relationship to ADR 0010 and ADR 0011

This ADR **narrowly amends ADR 0010** only for:

1. the relationship between inherited M6 `PAINT-05` order and atomic M9 groups;
2. deterministic group insertion/ordered membership; and
3. the mounted visual-subtree scope of node opacity and ordinary node shadows.

For those subjects, ADR 0012 controls. ADR 0011 continues to control path-fill,
stroke-closure, ellipse, and signed-shadow-spread/effect-bound semantics. Every other
ADR 0010 and ADR 0011 decision remains unchanged and authoritative. This is an
amendment relationship, not a second visual architecture.

## Decision

### M6 global item order is the canonical pre-group order

Runtime first derives the exact inherited M6 `PAINT-05` order over paint items:
stable ascending `(layer, mounted logical preorder, contribution-local order)`.

`SceneLayer` retains that meaning. M9 does not introduce a node z-index, group layer,
renderer resort, or parallel ordering key. When a publication contains no composition
groups, its paint ordering is exactly the accepted M6 ordering.

This ordered item sequence is the **pre-group order**. Group structure is resolved
from it by the framework-owned contraction rule below; a renderer never chooses a
different insertion position.

### Non-empty groups contract at their first descendant item

A non-empty composition group is one atomic stacking/compositing context.

Every paint item belongs to at most one immediate group. Every group has at most one
parent group. Group parentage is acyclic and therefore forms a snapshot-local forest
under the implicit scene root. Group references that are out of range, parent cycles,
or duplicate immediate ownership are invalid and reject/diagnose rather than being
repaired downstream.

Sibling groups may have descendant items interleaved in the pre-group order. That is
valid input to group composition and is exactly the ambiguity this ADR resolves.

For each non-empty group, runtime determines its **anchor** as the earliest pre-group
position occupied by any descendant paint item. Runtime recursively contracts inner
groups first. Within one parent, direct paint items and direct child groups are ordered
by the earliest pre-group position represented by that entry. The relative pre-group
order of members inside each group is preserved.

Contracting a group replaces all of its descendant items with one atomic parent entry
at that first-member anchor. Consequently, an outside item that previously sorted
between two descendants no longer interleaves the isolated group; when the group's
first descendant precedes that outside item, the complete group precedes it.

This loss of outside interleaving is the only intentional M9 qualification of M6
`PAINT-05`, and it is a necessary semantic consequence of explicit group isolation.
It is not backend behavior.

An empty group has empty composed coverage and effects and therefore has no meaningful
paint insertion point. Runtime may omit such a group from immutable publication.

### Node opacity and shadows isolate the mounted visual subtree

Node opacity and ordinary node shadows remain non-inherited style properties: a child
does not acquire the parent's style value or provenance.

Their **visual effect scope**, however, is the node's composed mounted visual subtree.
When a node requires group composition for opacity or ordinary shadows, runtime derives
a snapshot-local group whose descendants are the paint items of that node and its
mounted visual descendants. Descendant paint contributes to the ancestor group's
composed coverage even though the descendant did not inherit the ancestor style
property.

If a descendant node independently requires group composition, runtime creates a
nested group for that descendant. The nested result then participates as a child of
the ancestor group under the same contraction and effect-order rules.

Node outline is ordinary paint-only node visual output. Outline alone does not create
an isolation group and does not change the group ordering defined here.

Runtime-created node groups remain snapshot-local composition structure only. Their
presence does not create mounted identity, reconciliation keys, semantic identity,
widget state, lifecycle authority, or cross-publication identity.

### Explicit owner-local groups remain locally owned

A custom widget may author explicit owner-local composition groups only over its own
paint contribution members and nested local groups. Such a group cannot capture
arbitrary sibling or foreign mounted-owner paint by identity.

Runtime places those local groups inside any enclosing runtime-derived node-effect
group and applies the same pre-group ordering and first-member contraction rules.
The resulting immutable publication remains the sole surface-space group/order
authority.

### Existing group effect order is unchanged

ADR 0010's group composition order remains exact:

1. item clips constrain each item;
2. ordered child items/groups source-over compose;
3. group shadows derive from the composed child coverage, with ADR 0011 controlling
   signed spread and conservative effect bounds;
4. group clips constrain the complete group result including effects;
5. group opacity applies exactly once;
6. the complete group result source-over composes into its parent at the anchor frozen
   by this ADR.

Grouping, opacity, and shadows remain paint/effect/damage semantics only. They do not
change layout geometry, physical-hit geometry/order, directional-focus geometry, or
semantic geometry.

## Consequences

- M6 ordering remains directly observable and unchanged for ungrouped scenes.
- Creating an isolation group is explicitly a stacking-context operation; members no
  longer interleave with outside items after contraction.
- Runtime can derive group order solely from accepted paint facts and snapshot-local
  membership, without a new public group-layer vocabulary.
- Renderer implementations receive already-decided group structure/order and may use
  any disposable offscreen/cache strategy that realizes the same semantics.
- Node opacity/shadows can be implemented once over composed subtree coverage instead
  of being duplicated into widget paint or multiplied into child item opacity.

## Rejected alternatives

### Preserve arbitrary outside interleaving through an atomic group

Rejected because source-over group isolation cannot, in general, preserve an outside
item between two members while also applying effects/opacity once to the composed
group result.

### Multiply group opacity into child items

Rejected because overlapping child composition changes when opacity is applied per
child rather than once to their composed result.

### Add a node/group z-index or renderer-selected insertion key

Rejected for initial M9 because no additional ordering vocabulary is required to
resolve the ambiguity. The first-member anchor is deterministic from inherited M6
facts and keeps the correction bounded.

### Make `SceneLayer` group-local

Rejected because it would unnecessarily replace accepted M6 global layer semantics
for ungrouped scenes. M9 qualifies ordering only where explicit isolation makes the
old interleaving impossible.

## Conformance consequences

M9 conformance keeps exactly 25 rows and all existing statuses. `M9VIS-07` must prove:

- exact inherited M6 pre-group order;
- snapshot-local acyclic nesting and single immediate ownership;
- first-member anchoring and recursive contraction;
- sibling-group and outside-item pre-group interleaving cases;
- exact behavior with no groups and empty groups; and
- no mounted/semantic/cross-publication group identity leakage.

`M9VIS-08` must additionally prove that node opacity/shadows operate over composed
mounted visual-subtree coverage, nested effect nodes produce nested groups, group
opacity applies once, ADR 0011 shadow bounds remain conservative, and effects do not
leak into layout/hit/focus/semantic authority.

No row is promoted by accepting this architecture amendment.
