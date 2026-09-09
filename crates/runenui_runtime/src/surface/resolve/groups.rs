use std::collections::HashMap;

use runenui_core::{SceneLayer, SceneOpacity};

use crate::scene::{
    PaintSceneComposition, PaintSceneEntry, PaintSceneGroup, PaintSceneGroupId, PaintSceneItem,
};

use super::{CachedStyleFacts, SurfaceTopologySnapshot};

pub(super) type OrderedPaintItem = (SceneLayer, usize, usize, PaintSceneItem);

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

    let index_by_id = topology
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect::<HashMap<_, _>>();
    let parent = topology
        .nodes
        .iter()
        .map(|node| {
            node.parent
                .as_ref()
                .and_then(|parent| index_by_id.get(parent).copied())
        })
        .collect::<Vec<_>>();

    let requires_group = styles
        .resolutions
        .iter()
        .map(|resolution| {
            let computed = resolution.computed_style();
            computed.opacity() != SceneOpacity::OPAQUE || !computed.shadows().is_empty()
        })
        .collect::<Vec<_>>();

    // Candidate anchors are the earliest admitted pre-group descendant item.
    // A styled node with no admitted descendant paint is structurally empty and omitted.
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
    for (node, anchor) in candidate_anchor.iter().copied().enumerate() {
        let Some(anchor) = anchor else {
            continue;
        };
        let id = PaintSceneGroupId::new(group_nodes.len());
        group_for_node[node] = Some(id);
        group_nodes.push(node);
        group_anchors.push(anchor);
    }

    if group_nodes.is_empty() {
        let item_count = items.len();
        return (items, PaintSceneComposition::ungrouped(item_count));
    }

    let mut group_parents = vec![None; group_nodes.len()];
    for (group_index, node) in group_nodes.iter().copied().enumerate() {
        let mut cursor = parent[node];
        while let Some(ancestor) = cursor {
            if let Some(parent_group) = group_for_node[ancestor] {
                group_parents[group_index] = Some(parent_group);
                break;
            }
            cursor = parent[ancestor];
        }
    }

    let mut item_groups = vec![None; items.len()];
    for (item_index, owner) in item_owners.iter().copied().enumerate() {
        let mut cursor = Some(owner);
        while let Some(node) = cursor {
            if let Some(group) = group_for_node[node] {
                item_groups[item_index] = Some(group);
                break;
            }
            cursor = parent[node];
        }
        items[item_index].set_group(item_groups[item_index]);
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
        let id = PaintSceneGroupId::new(group_index);
        let entry = (group_anchors[group_index], PaintSceneEntry::group(id));
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
        .enumerate()
        .map(|(group_index, node)| {
            let computed = styles.resolutions[node].computed_style();
            PaintSceneGroup::new(
                group_parents[group_index],
                grouped_entries[group_index]
                    .drain(..)
                    .map(|(_, entry)| entry)
                    .collect(),
                Vec::new(),
                computed.opacity(),
                computed.shadows().to_vec(),
            )
        })
        .collect();

    (
        items,
        PaintSceneComposition::new(
            groups,
            root_entries.into_iter().map(|(_, entry)| entry).collect(),
        ),
    )
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
