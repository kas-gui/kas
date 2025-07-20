// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! AccessKit widget integration

use crate::{RoleCx, TextOrSource, Tile, TileExt, Window};
use accesskit::{Node, NodeId};

#[derive(Default)]
struct WalkCx {
    label: Option<String>,
    labelled_by: Option<NodeId>,
}

impl RoleCx for WalkCx {
    fn set_label_impl(&mut self, label: TextOrSource<'_>) {
        match label {
            TextOrSource::Borrowed(s) => self.label = Some(s.to_string()),
            TextOrSource::Owned(s) => self.label = Some(s),
            TextOrSource::Source(id) => self.labelled_by = Some(id.into()),
        }
    }
}

fn push_child(parent: &dyn Tile, index: usize, nodes: &mut Vec<(NodeId, Node)>) -> Option<NodeId> {
    if let Some(child) = parent.get_child(index) {
        let mut cx = WalkCx::default();
        let mut node = child.role(&mut cx).as_accesskit_node(child);
        parent.role_child_properties(&mut cx, index);

        if let Some(label) = cx.label.take() {
            node.set_label(label);
        } else if let Some(id) = cx.labelled_by.take() {
            node.set_labelled_by(vec![id]);
        }

        let children = push_all_children(child, nodes);
        if !children.is_empty() {
            node.set_children(children);
        }

        let id = child.id_ref().into();
        nodes.push((id, node));
        Some(id)
    } else {
        None
    }
}

fn push_all_children(tile: &dyn Tile, nodes: &mut Vec<(NodeId, Node)>) -> Vec<NodeId> {
    tile.child_indices()
        .into_iter()
        .flat_map(|index| push_child(tile, index, nodes))
        .collect()
}

/// Returns a list of nodes and the root node's identifier
pub(crate) fn window_nodes<Data: 'static>(root: &Window<Data>) -> (Vec<(NodeId, Node)>, NodeId) {
    let mut nodes = vec![];
    let mut children = push_all_children(root.as_tile(), &mut nodes);

    for popup in root.iter_popups() {
        if let Some(tile) = root.find_tile(&popup.id) {
            let index = crate::POPUP_INNER_INDEX;
            if let Some(id) = push_child(tile, index, &mut nodes) {
                children.push(id);
            }
        }
    }

    let mut cx = WalkCx::default();
    let mut node = root.role(&mut cx).as_accesskit_node(root);
    node.set_children(children);

    if let Some(label) = cx.label.take() {
        node.set_label(label);
    } else if let Some(id) = cx.labelled_by.take() {
        node.set_labelled_by(vec![id]);
    }

    let root_id = root.id_ref().into();
    nodes.push((root_id, node));
    (nodes, root_id)
}
