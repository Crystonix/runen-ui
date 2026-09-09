use std::collections::HashMap;

use runenui_core::{SceneLayer, SceneOpacity};

use crate::scene::{
    PaintSceneComposition, PaintSceneEntry, PaintSceneGroup, PaintSceneGroupId, PaintSceneItem,
};

use super::{CachedStyleFacts, SurfaceTopologySnapshot};

pub(super) type OrderedPaintItem = (SceneLayer, usize, usize, PaintSceneItem);

struct StaticGroupPlan {
    group_nodes: Vec<usize>,
    group_anchors: Vec<usize>,
    group_parents: Vec<Option<PaintSceneGroupId>>,
    item_groups: Vec<Option<PaintSceneGroupId>>,
}

fn topology_parents(topology: &SurfaceTopologySnapshot) -> Vec<Option<usize>> {
    let index_by_id = topology
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect::<HashMap<_, _>>();
    topology
        .nodes
        .iter()
        .map(|node| {
            node.parent
                .as_ref()
                .and_then(|parent| index_by_id.get(parent).copied())
        })
        .collect()
}

const fn nearest_group(
    mut cursor: Option<usize>,
    parent: &[Option<usize>],
    group_for_node: &[Option<PaintSceneGroupId>],
) -> Option<PaintSceneGroupId> {
    while let Some(node) = cursor {
        if let Some(group) = group_for_node[node] {
            return Some(group);
        }
        cursor = parent[node];
    }
    None
}

fn derive_static_group_plan(
    topology: &SurfaceTopologySnapshot,
    styles: &CachedStyleFacts,
    item_owners: &[usize],
) -> StaticGroupPlan {
    let parent = topology_parents(topology);
    let requires_group = styles
        .resolutions
        .iter()
        .map(|resolution| {
            let computed = resolution.computed_style();
            computed.opacity() != SceneOpacity::OPAQUE || !computed.shadows().is_empty()
        })
        .collect::<Vec<_>>();

    let mut candidate_anchor = vec![None; topology.nodes.len()];
    for (item_index, owner) in item_owners.iter().copied().enumerate() {
        let mut cursor = Some(owner);
        while let Some(node) = cursor {
            if requires_group[node] {
                candidate_anchor[node] = Some(
                    candidate_anchor[node]
                        .map_or(item_index, |anchor: usize| anchor.min(item_index)),
                );
            }
            cursor = parent[node];
        }
    }

    let mut group_for_node = vec![None; topology.nodes.len()];
    let mut group_nodes = Vec::new();
    let mut group_anchors = Vec::new();
    for (node, anchor) in candidate_anchor.into_iter().enumerate() {
        let Some(anchor) = anchor else {
            continue;
        };
        let group = PaintSceneGroupId::new(group_nodes.len());
        group_for_node[node] = Some(group);
        group_nodes.push(node);
        group_anchors.push(anchor);
    }

    let group_parents = group_nodes
        .iter()
        .copied()
        .map(|node| nearest_group(parent[node], &parent, &group_for_node))
        .collect();
    let item_groups = item_owners
        .iter()
        .copied()
        .map(|owner| nearest_group(Some(owner), &parent, &group_for_node))
        .collect();

    StaticGroupPlan {
        group_nodes,
        group_anchors,
        group_parents,
        item_groups,
    }
}

/// Applies ADR 0012 static node-effect grouping to an already M6-ordered item list.
///
/// The input order is canonical pre-group order. Returned `items` preserve it exactly;
/// only the separate composition structure contracts isolated descendants.
pub(super) fn derive_static_node_effect_groups(
    topology: &SurfaceTopologySnapshot,
    styles: &CachedStyleFacts,
    ordered: Vec<OrderedPaintItem>,
) -> (Vec<PaintSceneItem>, PaintSceneComposition) {
    debug_assert_eq!(topology.nodes.len(), styles.resolutions.len());

    let item_owners = ordered
        .iter()
        .map(|(_, mounted_preorder, _, _)| *mounted_preorder)
        .collect::<Vec<_>>();
    let mut items = ordered
        .into_iter()
        .map(|(_, _, _, item)| item)
        .collect::<Vec<_>>();
    if items.is_empty() || topology.nodes.is_empty() {
        let item_count = items.len();
        return (items, PaintSceneComposition::ungrouped(item_count));
    }

    let StaticGroupPlan {
        group_nodes,
        group_anchors,
        group_parents,
        item_groups,
    } = derive_static_group_plan(topology, styles, &item_owners);
    if group_nodes.is_empty() {
        let item_count = items.len();
        return (items, PaintSceneComposition::ungrouped(item_count));
    }

    for (item, group) in items.iter_mut().zip(&item_groups) {
        item.set_group(*group);
    }

    let mut grouped_entries = vec![Vec::<(usize, PaintSceneEntry)>::new(); group_nodes.len()];
    let mut root_entries = Vec::<(usize, PaintSceneEntry)>::new();
    for (item_index, group) in item_groups.iter().copied().enumerate() {
        let entry = (item_index, PaintSceneEntry::item(item_index));
        if let Some(group) = group {
            grouped_entries[group.index()].push(entry);
        } else {
            root_entries.push(entry);
        }
    }
    for (group_index, parent_group) in group_parents.iter().copied().enumerate() {
        let group = PaintSceneGroupId::new(group_index);
        let entry = (group_anchors[group_index], PaintSceneEntry::group(group));
        if let Some(parent_group) = parent_group {
            grouped_entries[parent_group.index()].push(entry);
        } else {
            root_entries.push(entry);
        }
    }
    for entries in &mut grouped_entries {
        entries.sort_by_key(|(anchor, _)| *anchor);
    }
    root_entries.sort_by_key(|(anchor, _)| *anchor);

    let groups = group_nodes
        .into_iter()
        .zip(grouped_entries)
        .enumerate()
        .map(|(group_index, (node, entries))| {
            let computed = styles.resolutions[node].computed_style();
            PaintSceneGroup::new(
                group_parents[group_index],
                entries.into_iter().map(|(_, entry)| entry).collect(),
                Vec::new(),
                computed.opacity(),
                computed.shadows().to_vec(),
            )
        })
        .collect();
    let root_entries = root_entries.into_iter().map(|(_, entry)| entry).collect();
    (items, PaintSceneComposition::new(groups, root_entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_member_contraction_ordering_is_stable_for_entries() {
        let group = PaintSceneGroupId::new(0);
        let mut entries = [
            (2, PaintSceneEntry::item(2)),
            (0, PaintSceneEntry::group(group)),
            (1, PaintSceneEntry::item(1)),
        ];
        entries.sort_by_key(|(anchor, _)| *anchor);
        assert_eq!(entries[0].1.group_id(), Some(group));
        assert_eq!(entries[1].1.item_index(), Some(1));
        assert_eq!(entries[2].1.item_index(), Some(2));
    }
}
