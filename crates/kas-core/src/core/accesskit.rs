// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! AccessKit utilities

use super::{Collection, Tile};
use crate::cast::Cast;
use accesskit::{Node, NodeId};

/// Context for accessibility shadow-tree recursion
// NOTE: invariant: 0 <= self.start_unclaimed <= self.nodes.len()
#[derive(Debug)]
pub struct AccessKitCx {
    nodes: Vec<(NodeId, Node)>,
    start_unclaimed: usize,
}

impl AccessKitCx {
    pub(crate) fn new() -> Self {
        AccessKitCx {
            nodes: Vec::new(),
            start_unclaimed: 0,
        }
    }

    pub(crate) fn start_unclaimed(&self) -> usize {
        self.start_unclaimed
    }

    pub(crate) fn take_nodes(self) -> Vec<(NodeId, Node)> {
        self.nodes
    }

    /// Push a [`Tile`] to the list of updated nodes
    ///
    /// This method calls [`Tile::accesskit_recurse`], assuming that all newly
    /// added nodes are children of `tile`. This method then calls
    /// [`Tile::accesskit_node`], setting `bounds` and `children`, and adds the
    /// [`Node`] to the list of new or updated nodes in the accessibility tree.
    #[inline]
    pub fn push(&mut self, tile: &dyn Tile) {
        self._push_with(tile, None);
    }

    /// Push a [`Tile`], invoking a callback on the tile's [`accesskit::Node`]
    ///
    /// This variant of [`Self::push`] may be used to attach additional
    /// information, for example the `labelled_by` property.
    pub fn push_with(&mut self, tile: &dyn Tile, mut cb: impl FnMut(&mut Node)) {
        self._push_with(tile, Some(&mut cb));
    }

    fn _push_with(&mut self, tile: &dyn Tile, cb: Option<&mut dyn FnMut(&mut Node)>) {
        assert!(
            tile.id_ref().is_valid(),
            "unconfigured widgets should not be included"
        );

        // Invariant at fn start/end: nodes in self.nodes[..self.start_unclaimed] have a parent
        // This is the number of unclaimed children (not ours):
        let extra = self.nodes.len() - self.start_unclaimed;

        // Recursion may place additional claimed children in self.nodes[..self.start_unclaimed]
        // (increases self.start_unclaimed) and unclaimed children in self.nodes[self.start_unclaimed+extra..]:
        tile.accesskit_recurse(self);
        let start = self.start_unclaimed + extra;

        if let Some(mut node) = tile.accesskit_node() {
            node.set_bounds(tile.rect().cast());

            if start < self.nodes.len() {
                // Claim children:
                node.set_children(
                    self.nodes[start..]
                        .iter()
                        .map(|pair| pair.0)
                        .collect::<Vec<_>>(),
                );
                // Move self.nodes[self.start_unclaimed..self.start_unclaimed+extra]
                // to self.nodes[len-extra..len] maintaining order:
                let unclaimed_start = self.nodes.len() - extra;
                for i in (0..extra).rev() {
                    self.nodes
                        .swap(self.start_unclaimed + i, unclaimed_start + i);
                }
                self.start_unclaimed = unclaimed_start;
            }

            if let Some(cb) = cb {
                cb(&mut node);
            }
            self.nodes.push((tile.id_ref().into(), node));
        } else {
            // Note that there may be unclaimed children; we could synthesise a
            // node with Role::GenericContainer, but it's also fine to leave
            // these unclaimed for our parent node.
        }
    }

    /// Extend self from a collection
    pub fn extend_collection<C: Collection>(&mut self, collection: &C) {
        self.nodes.reserve(collection.len());
        for tile in collection.iter_tile(..) {
            self.push(tile);
        }
    }

    /// Extend self from an iterator over tiles
    ///
    /// This has the same effect as calling [`Self::push`] on each tile.
    pub fn extend<'a, I: IntoIterator<Item = &'a dyn Tile>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        if let Some(ub) = iter.size_hint().1 {
            self.nodes.reserve(ub);
        }
        for tile in iter {
            self.push(tile);
        }
    }
}
