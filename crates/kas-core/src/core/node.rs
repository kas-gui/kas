// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Node API for widgets

use super::Widget;
use crate::event::{ConfigMgr, Event, EventMgr, Response};
use crate::geom::{Coord, Rect};
use crate::layout::{AxisInfo, SizeRules};
use crate::theme::{DrawMgr, SizeMgr};
use crate::{Erased, Layout, NavAdvance, WidgetId};

#[cfg(not(feature = "unsafe_node"))]
trait NodeMutT {
    fn id_ref(&self) -> &WidgetId;
    fn rect(&self) -> Rect;

    fn clone_node_mut(&mut self) -> NodeMut<'_>;
    fn as_layout(&self) -> &dyn Layout;

    fn num_children(&self) -> usize;
    fn find_child_index(&self, id: &WidgetId) -> Option<usize>;
    fn for_child_mut_impl(&mut self, index: usize, f: Box<dyn FnOnce(NodeMut<'_>) + '_>);

    fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules;
    fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect);

    fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize>;
    fn find_id(&mut self, coord: Coord) -> Option<WidgetId>;
    fn _draw(&mut self, draw: DrawMgr);

    fn _configure(&mut self, cx: &mut ConfigMgr, id: WidgetId);
    fn _update(&mut self, cx: &mut ConfigMgr);

    fn _send(&mut self, cx: &mut EventMgr, id: WidgetId, disabled: bool, event: Event) -> Response;
    fn _replay(&mut self, cx: &mut EventMgr, id: WidgetId, msg: Erased);
    fn _nav_next(
        &mut self,
        cx: &mut EventMgr,
        focus: Option<&WidgetId>,
        advance: NavAdvance,
    ) -> Option<WidgetId>;
}
#[cfg(not(feature = "unsafe_node"))]
impl<'a, T> NodeMutT for (&'a mut dyn Widget<Data = T>, &'a T) {
    fn id_ref(&self) -> &WidgetId {
        self.0.id_ref()
    }
    fn rect(&self) -> Rect {
        self.0.rect()
    }

    fn clone_node_mut(&mut self) -> NodeMut<'_> {
        NodeMut::new(self.0, self.1)
    }
    fn as_layout(&self) -> &dyn Layout {
        self.0.as_layout()
    }

    fn num_children(&self) -> usize {
        self.0.num_children()
    }
    fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
        self.0.find_child_index(id)
    }

    fn for_child_mut_impl(&mut self, index: usize, f: Box<dyn FnOnce(NodeMut<'_>) + '_>) {
        self.0.for_child_mut_impl(self.1, index, f);
    }

    fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        self.0.size_rules(size_mgr, axis)
    }
    fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
        self.0.set_rect(mgr, rect);
    }

    fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        self.0.nav_next(reverse, from)
    }
    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        self.0.find_id(coord)
    }
    fn _draw(&mut self, mut draw: DrawMgr) {
        draw.recurse(&mut self.0);
    }

    fn _configure(&mut self, cx: &mut ConfigMgr, id: WidgetId) {
        self.0._configure(self.1, cx, id);
    }
    fn _update(&mut self, cx: &mut ConfigMgr) {
        self.0._update(self.1, cx);
    }

    fn _send(&mut self, cx: &mut EventMgr, id: WidgetId, disabled: bool, event: Event) -> Response {
        self.0._send(self.1, cx, id, disabled, event)
    }
    fn _replay(&mut self, cx: &mut EventMgr, id: WidgetId, msg: Erased) {
        self.0._replay(self.1, cx, id, msg);
    }
    fn _nav_next(
        &mut self,
        cx: &mut EventMgr,
        focus: Option<&WidgetId>,
        advance: NavAdvance,
    ) -> Option<WidgetId> {
        self.0._nav_next(self.1, cx, focus, advance)
    }
}

/// Public API over a contextualized mutable widget
///
/// Note: this type has no publically supported utility over [`Node`].
/// It is, however, required for Kas's internals.
#[cfg(feature = "unsafe_node")]
pub struct NodeMut<'a>(&'a mut dyn Widget<Data = ()>, &'a ());
#[cfg(not(feature = "unsafe_node"))]
pub struct NodeMut<'a>(Box<dyn NodeMutT + 'a>);

impl<'a> NodeMut<'a> {
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
                unsafe { NodeMut(transmute(widget), transmute(data)) }
            } else {
                NodeMut(Box::new((widget, data)))
            }
        }
    }

    /// Reborrow with a new lifetime
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    #[inline(always)]
    pub fn re<'b>(&'b mut self) -> NodeMut<'b>
    where
        'a: 'b,
    {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                NodeMut(self.0, self.1)
            } else {
                self.0.clone_node_mut()
            }
        }
    }

    /// Reborrow as a `dyn Layout`
    pub fn as_layout(&self) -> &dyn Layout {
        self.0.as_layout()
    }

    /// Get the widget's identifier
    #[inline]
    pub fn id_ref(&self) -> &WidgetId {
        self.0.id_ref()
    }

    /// Get the widget's identifier
    #[inline]
    pub fn id(&self) -> WidgetId {
        self.id_ref().clone()
    }

    /// Test widget identifier for equality
    ///
    /// This method may be used to test against `WidgetId`, `Option<WidgetId>`
    /// and `Option<&WidgetId>`.
    #[inline]
    pub fn eq_id<T>(&self, rhs: T) -> bool
    where
        WidgetId: PartialEq<T>,
    {
        *self.id_ref() == rhs
    }

    /// Check whether `id` is self or a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    pub fn is_ancestor_of(&self, id: &WidgetId) -> bool {
        self.id().is_ancestor_of(id)
    }

    /// Check whether `id` is not self and is a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    pub fn is_strict_ancestor_of(&self, id: &WidgetId) -> bool {
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

    /// Run `f` on some child by index and, if valid, return the result.
    ///
    /// Calls the closure and returns `Some(result)` exactly when
    /// `index < self.num_children()`.
    pub fn for_child<R>(&mut self, index: usize, f: impl FnOnce(NodeMut<'_>) -> R) -> Option<R> {
        let mut result = None;
        let out = &mut result;
        let f: Box<dyn for<'b> FnOnce(NodeMut<'b>)> = Box::new(|node| {
            *out = Some(f(node));
        });
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0.for_child_mut_impl(self.1, index, f);
            } else {
                self.0.for_child_mut_impl(index, f);
            }
        }
        result
    }

    /// Run a `f` on all children
    pub fn for_children(&mut self, mut f: impl FnMut(NodeMut<'_>)) {
        for index in 0..self.0.num_children() {
            // NOTE: for_child_mut_impl takes FnOnce hence we must wrap the closure
            let f = &mut f;
            let f: Box<dyn for<'b> FnOnce(NodeMut<'b>)> = Box::new(|node| {
                f(node);
            });
            cfg_if::cfg_if! {
                if #[cfg(feature = "unsafe_node")] {
                    self.0.for_child_mut_impl(self.1, index, f);
                } else {
                    self.0.for_child_mut_impl(index, f);
                }
            }
        }
    }

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// If `Some(index)` is returned, this is *probably* but not guaranteed
    /// to be a valid child index.
    #[inline]
    pub fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
        self.0.find_child_index(id)
    }

    /// Find the descendant with this `id`, if any, and call `cb` on it
    ///
    /// Returns `Some(result)` if and only if node `id` was found.
    pub fn for_id<F: FnOnce(NodeMut<'_>) -> T, T>(&mut self, id: &WidgetId, cb: F) -> Option<T> {
        if let Some(index) = self.find_child_index(id) {
            self.for_child(index, |mut node| node.for_id(id, cb))
                .unwrap()
        } else if self.eq_id(id) {
            Some(cb(self.re()))
        } else {
            None
        }
    }
}

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl<'a> NodeMut<'a> {
    /// Get size rules for the given axis
    pub(crate) fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        self.0.size_rules(size_mgr, axis)
    }

    /// Set size and position
    pub(crate) fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
        self.0.set_rect(mgr, rect);
    }

    /// Navigation in spatial order
    pub(crate) fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        self.0.nav_next(reverse, from)
    }

    /// Translate a coordinate to a [`WidgetId`]
    pub(crate) fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        self.0.find_id(coord)
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "unsafe_node")] {
            /// Draw a widget and its children
            pub(crate) fn _draw(&mut self, mut draw: DrawMgr) {
                draw.recurse(&mut self.0);
            }
        } else {
            /// Draw a widget and its children
            pub(crate) fn _draw(&mut self, draw: DrawMgr) {
                self.0._draw(draw);
            }
        }
    }

    /// Internal method: configure recursively
    pub(crate) fn _configure(&mut self, cx: &mut ConfigMgr, id: WidgetId) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._configure(self.1, cx, id);
            } else {
                self.0._configure(cx, id);
            }
        }
    }

    /// Internal method: update recursively
    pub(crate) fn _update(&mut self, cx: &mut ConfigMgr) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._update(self.1, cx);
            } else {
                self.0._update(cx);
            }
        }
    }

    /// Internal method: send recursively
    pub(crate) fn _send(
        &mut self,
        cx: &mut EventMgr,
        id: WidgetId,
        disabled: bool,
        event: Event,
    ) -> Response {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._send(self.1, cx, id, disabled, event)
            } else {
                self.0._send(cx, id, disabled, event)
            }
        }
    }

    /// Internal method: replay recursively
    pub(crate) fn _replay(&mut self, cx: &mut EventMgr, id: WidgetId, msg: Erased) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._replay(self.1, cx, id, msg);
            } else {
                self.0._replay(cx, id, msg);
            }
        }
    }

    /// Internal method: search for the previous/next navigation target
    // NOTE: public on account of ListView
    pub fn _nav_next(
        &mut self,
        cx: &mut EventMgr,
        focus: Option<&WidgetId>,
        advance: NavAdvance,
    ) -> Option<WidgetId> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "unsafe_node")] {
                self.0._nav_next(self.1, cx, focus, advance)
            } else {
                self.0._nav_next(cx, focus, advance)
            }
        }
    }
}
