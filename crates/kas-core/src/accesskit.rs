// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! AccessKit widget integration

use crate::cast::Cast;
use crate::geom::Offset;
use crate::window::{POPUP_INNER_INDEX, Window};
use crate::{Role, RoleCx, TextOrSource, Tile, TileExt};
use accesskit::{Action, Node, NodeId};

pub(crate) fn apply_scroll_props_to_node(offset: Offset, max_offset: Offset, node: &mut Node) {
    if offset.1 < max_offset.1 {
        node.add_action(Action::ScrollDown);
    }
    if offset.0 > 0 {
        node.add_action(Action::ScrollLeft);
    }
    if offset.0 < max_offset.0 {
        node.add_action(Action::ScrollRight);
    }
    if offset.1 > 0 {
        node.add_action(Action::ScrollUp);
    }
    node.add_action(Action::SetScrollOffset);
    node.set_scroll_x(offset.0.cast());
    node.set_scroll_y(offset.1.cast());
    node.set_scroll_x_min(0.0);
    node.set_scroll_y_min(0.0);
    node.set_scroll_x_max(max_offset.0.cast());
    node.set_scroll_y_max(max_offset.1.cast());
    node.set_clips_children();
}

#[derive(Default)]
struct WalkCx {
    label: Option<String>,
    labelled_by: Option<NodeId>,
    scroll_offset: Option<(Offset, Offset)>,
}

impl WalkCx {
    fn apply_to_node(mut self, node: &mut Node) {
        if let Some(label) = self.label.take() {
            node.set_label(label);
        } else if let Some(id) = self.labelled_by.take() {
            node.set_labelled_by(vec![id]);
        }
        if let Some((offset, max_offset)) = self.scroll_offset.take() {
            apply_scroll_props_to_node(offset, max_offset, node);
        }
    }
}

impl RoleCx for WalkCx {
    fn set_label_impl(&mut self, label: TextOrSource<'_>) {
        match label {
            TextOrSource::Borrowed(s) => self.label = Some(s.to_string()),
            TextOrSource::Owned(s) => self.label = Some(s),
            TextOrSource::Source(id) => self.labelled_by = Some(id.into()),
        }
    }

    fn set_scroll_offset(&mut self, offset: Offset, max_offset: Offset) {
        self.scroll_offset = Some((offset, max_offset));
    }
}

fn push_child(
    parent: &dyn Tile,
    index: usize,
    nodes: &mut Vec<(NodeId, Node)>,
    has_scrollable_parent: bool,
) -> Option<NodeId> {
    if let Some(child) = parent.get_child(index) {
        let mut cx = WalkCx::default();
        let role = child.role(&mut cx);
        let child_is_scrollable = matches!(role, Role::ScrollRegion { .. });
        parent.role_child_properties(&mut cx, index);

        let mut node = role.as_accesskit_node(child);
        cx.apply_to_node(&mut node);

        if has_scrollable_parent {
            node.add_action(Action::ScrollIntoView);
        }

        let children =
            push_all_children(child, nodes, has_scrollable_parent || child_is_scrollable);
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

fn push_all_children(
    tile: &dyn Tile,
    nodes: &mut Vec<(NodeId, Node)>,
    has_scrollable_parent: bool,
) -> Vec<NodeId> {
    tile.child_indices()
        .into_iter()
        .flat_map(|index| push_child(tile, index, nodes, has_scrollable_parent))
        .collect()
}

/// Returns a list of nodes and the root node's identifier
pub(crate) fn window_nodes<Data: 'static>(root: &Window<Data>) -> (Vec<(NodeId, Node)>, NodeId) {
    let mut nodes = vec![];
    let mut children = push_all_children(root.as_tile(), &mut nodes, false);

    for popup in root.iter_popups() {
        if let Some(tile) = root.find_tile(&popup.id) {
            let index = POPUP_INNER_INDEX;
            if let Some(id) = push_child(tile, index, &mut nodes, false) {
                children.push(id);
            }
        }
    }

    let mut cx = WalkCx::default();
    let mut node = root.role(&mut cx).as_accesskit_node(root);
    cx.apply_to_node(&mut node);
    node.set_children(children);

    let root_id = root.id_ref().into();
    nodes.push((root_id, node));
    (nodes, root_id)
}
