// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! AccessKit utilities

use super::Tile;
use crate::cast::Cast;
use accesskit::{Node, NodeId};

/// Context for accessibility shadow-tree recursion
#[derive(Debug)]
pub struct AccessKitCx(Vec<(NodeId, Node)>, usize);

impl AccessKitCx {
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
        // Invariant at fn start/end: nodes in self.0[..self.1] have a parent
        // This is the number of unclaimed children (not ours):
        let extra = self.0.len() - self.1;

        // Recursion may place additional claimed children in self.0[..self.1]
        // (increases self.1) and unclaimed children in self.0[self.1+extra..]:
        tile.accesskit_recurse(self);
        let start = self.1 + extra;

        if let Some(mut node) = tile.accesskit_node() {
            node.set_bounds(tile.rect().cast());

            if start < self.0.len() {
                // Claim children, and move under self.1:
                node.set_children(
                    self.0[start..]
                        .iter()
                        .map(|pair| pair.0)
                        .collect::<Vec<_>>(),
                );
                let unclaimed_start = self.0.len() - extra;
                for i in 0..(unclaimed_start - self.1) {
                    self.0.swap(self.1 + i, unclaimed_start + i);
                }
                self.1 = unclaimed_start;
            }

            if let Some(cb) = cb {
                cb(&mut node);
            }
            self.0.push((tile.id_ref().into(), node));
        } else {
            // Note that there may be unclaimed children; we could synthesise a
            // node with Role::GenericContainer, but it's also fine to leave
            // these unclaimed for our parent node.
        }
    }

    /// Extend self from an iterator over tiles
    ///
    /// This has the same effect as calling [`Self::push`] on each tile.
    pub fn extend<'a, I: IntoIterator<Item = &'a dyn Tile>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        if let Some(ub) = iter.size_hint().1 {
            self.0.reserve(ub);
        }
        for tile in iter {
            self.push(tile);
        }
    }
}
