// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Node API for widgets

use super::Widget;
use crate::event::{ConfigCx, Event, EventCx, IsUsed};
use crate::geom::Rect;
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::theme::SizeCx;
use crate::{Id, NavAdvance, Tile};

#[cfg(not(feature = "unsafe_node"))]
trait NodeT {
    fn id_ref(&self) -> &Id;
    fn rect(&self) -> Rect;

    fn clone_node(&mut self) -> Node<'_>;
    fn as_tile(&self) -> &dyn Tile;

    fn num_children(&self) -> usize;
    fn find_child_index(&self, id: &Id) -> Option<usize>;
    fn child_node(&mut self, index: usize) -> Option<Node<'_>>;

    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules;
    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints);

    fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize>;

    fn _configure(&mut self, cx: &mut ConfigCx, id: Id);
    fn _update(&mut self, cx: &mut ConfigCx);

    fn _send(&mut self, cx: &mut EventCx, id: Id, event: Event) -> IsUsed;
    fn _replay(&mut self, cx: &mut EventCx, id: Id);
    fn _nav_next(
        &mut self,
        cx: &mut ConfigCx,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id>;
}
#[cfg(not(feature = "unsafe_node"))]
impl<'a, T> NodeT for (&'a mut dyn Widget<Data = T>, &'a T) {
    fn id_ref(&self) -> &Id {
        self.0.id_ref()
    }
    fn rect(&self) -> Rect {
        self.0.rect()
    }

    fn clone_node(&mut self) -> Node<'_> {
        Node::new(self.0, self.1)
    }
    fn as_tile(&self) -> &dyn Tile {
        self.0.as_tile()
    }

    fn num_children(&self) -> usize {
        self.0.num_children()
    }
    fn find_child_index(&self, id: &Id) -> Option<usize> {
        self.0.find_child_index(id)
    }

    fn child_node(&mut self, index: usize) -> Option<Node<'_>> {
        self.0.child_node(self.1, index)
    }

    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.0.size_rules(sizer, axis)
    }
    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        self.0.set_rect(cx, rect, hints);
    }

    fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        self.0.nav_next(reverse, from)
    }

    fn _configure(&mut self, cx: &mut ConfigCx, id: Id) {
        self.0._configure(cx, self.1, id);
    }
    fn _update(&mut self, cx: &mut ConfigCx) {
        self.0._update(cx, self.1);
    }

    fn _send(&mut self, cx: &mut EventCx, id: Id, event: Event) -> IsUsed {
        self.0._send(cx, self.1, id, event)
    }
    fn _replay(&mut self, cx: &mut EventCx, id: Id) {
        self.0._replay(cx, self.1, id);
    }
    fn _nav_next(
        &mut self,
        cx: &mut ConfigCx,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id> {
        self.0._nav_next(cx, self.1, focus, advance)
    }
}

/// Type-erased widget with input data
///
/// This type is a `&mut dyn Widget<Data = A>` paired with input data `&A`,
/// where the type `A` is erased.
///
/// The default implementation of this type uses a boxed trait object.
/// The `unsafe_node` feature enables a more efficient unboxed implementation
/// (this must make assumptions about VTables beyond what Rust specifies, thus
/// lacks even the usual programmer-provided verification of `unsafe` code).
pub struct Node<'a>(
    #[cfg(not(feature = "unsafe_node"))] Box<dyn NodeT + 'a>,
    #[cfg(feature = "unsafe_node")] &'a mut dyn Widget<Data = ()>,
    #[cfg(feature = "unsafe_node")] &'a (),
);

impl<'a> Node<'a> {
    /// Construct
    #[inline(always)]
    pub fn new<T: 'a>(widget: &'a mut dyn Widget<Data = T>, data: &'a T) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                // Safety: since the vtable for dyn Widget<Data = T> only uses T as &T
                // and T: Sized, the vtable should be equivalent for all T.
                // We ensure here that the type of `data` matches that used by `widget`.
                // NOTE: This makes assumptions beyond Rust's specification.
                use std::mem::transmute;
                unsafe { Node(transmute(widget), transmute(data)) }
            } else {
                Node(Box::new((widget, data)))
            }
        }
    }

    /// Reborrow with a new lifetime
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    #[inline(always)]
    pub fn re<'b>(&'b mut self) -> Node<'b>
    where
        'a: 'b,
    {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                Node(self.0, self.1)
            } else {
                self.0.clone_node()
            }
        }
    }

    /// Reborrow as a `dyn Tile`
    pub fn as_tile(&self) -> &dyn Tile {
        self.0.as_tile()
    }

    /// Get the widget's identifier
    #[inline]
    pub fn id_ref(&self) -> &Id {
        self.0.id_ref()
    }

    /// Get the widget's identifier
    #[inline]
    pub fn id(&self) -> Id {
        self.id_ref().clone()
    }

    /// Test widget identifier for equality
    ///
    /// This method may be used to test against `Id`, `Option<Id>`
    /// and `Option<&Id>`.
    #[inline]
    pub fn eq_id<T>(&self, rhs: T) -> bool
    where
        Id: PartialEq<T>,
    {
        *self.id_ref() == rhs
    }

    /// Check whether `id` is self or a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    pub fn is_ancestor_of(&self, id: &Id) -> bool {
        self.id().is_ancestor_of(id)
    }

    /// Check whether `id` is not self and is a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    pub fn is_strict_ancestor_of(&self, id: &Id) -> bool {
        !self.eq_id(id) && self.id().is_ancestor_of(id)
    }

    /// Get the widget's region, relative to its parent.
    #[inline]
    pub fn rect(&self) -> Rect {
        self.0.rect()
    }

    /// Get the number of child widgets
    ///
    /// Every value in the range `0..self.num_children()` is a valid child
    /// index.
    #[inline]
    pub fn num_children(&self) -> usize {
        self.0.num_children()
    }

    /// Run `f` on some child by index.
    ///
    /// Calls the closure exactly when `index < self.num_children()`.
    #[inline(always)]
    pub fn get_child(&mut self, index: usize) -> Option<Node<'_>> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0.child_node(self.1, index)
            } else {
                self.0.child_node(index)
            }
        }
    }

    /// Run a `f` on all children
    #[inline(always)]
    pub fn for_children(&mut self, mut f: impl FnMut(Node<'_>)) {
        for index in 0..self.0.num_children() {
            let f: Box<dyn for<'b> FnOnce(Node<'b>)> = Box::new(&mut f);
            let opt_node = {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "unsafe_node")] {
                        self.0.child_node(self.1, index)
                    } else {
                        self.0.child_node(index)
                    }
                }
            };
            if let Some(node) = opt_node {
                f(node);
            }
        }
    }

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// If `Some(index)` is returned, this is *probably* but not guaranteed
    /// to be a valid child index.
    #[inline]
    pub fn find_child_index(&self, id: &Id) -> Option<usize> {
        self.0.find_child_index(id)
    }

    /// Find the descendant with this `id`, if any, and call `cb` on it
    ///
    /// Returns `Some(result)` if and only if node `id` was found.
    #[inline(always)]
    pub fn find_node<F: FnOnce(Node<'_>) -> T, T>(&mut self, id: &Id, cb: F) -> Option<T> {
        if let Some(index) = self.find_child_index(id) {
            if let Some(mut node) = self.get_child(index) {
                node.find_node(id, cb)
            } else {
                debug_assert!(false);
                None
            }
        } else if self.eq_id(id) {
            Some(cb(self.re()))
        } else {
            None
        }
    }
}

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
impl<'a> Node<'a> {
    /// Get size rules for the given axis
    pub(crate) fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.0.size_rules(sizer, axis)
    }

    /// Set size and position
    pub(crate) fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        self.0.set_rect(cx, rect, hints);
    }

    /// Navigation in spatial order
    pub(crate) fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        self.0.nav_next(reverse, from)
    }

    /// Internal method: configure recursively
    pub(crate) fn _configure(&mut self, cx: &mut ConfigCx, id: Id) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._configure(cx, self.1, id);
            } else {
                self.0._configure(cx, id);
            }
        }
    }

    /// Internal method: update recursively
    pub(crate) fn _update(&mut self, cx: &mut ConfigCx) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._update(cx, self.1);
            } else {
                self.0._update(cx);
            }
        }
    }

    /// Internal method: send recursively
    pub(crate) fn _send(&mut self, cx: &mut EventCx, id: Id, event: Event) -> IsUsed {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._send(cx, self.1, id, event)
            } else {
                self.0._send(cx, id, event)
            }
        }
    }

    /// Internal method: replay recursively
    pub(crate) fn _replay(&mut self, cx: &mut EventCx, id: Id) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._replay(cx, self.1, id);
            } else {
                self.0._replay(cx, id);
            }
        }
    }

    /// Internal method: search for the previous/next navigation target
    // NOTE: public on account of ListView
    pub fn _nav_next(
        &mut self,
        cx: &mut ConfigCx,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._nav_next(cx, self.1, focus, advance)
            } else {
                self.0._nav_next(cx, focus, advance)
            }
        }
    }
}
