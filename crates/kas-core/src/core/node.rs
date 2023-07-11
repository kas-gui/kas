// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Node API for widgets

use super::Widget;
use crate::event::{ConfigMgr, Event, EventMgr, Response};
use crate::geom::{Coord, Offset, Rect};
use crate::layout::{AxisInfo, SizeRules};
use crate::theme::{DrawMgr, SizeMgr};
use crate::util::IdentifyWidget;
use crate::{Erased, NavAdvance, WidgetId};

/// Public API over a mutable widget
pub struct Node<'a>(&'a dyn Widget<Data = ()>, &'a ());

impl<'a> Node<'a> {
    /// Construct
    #[inline(always)]
    pub fn new<T: 'a>(widget: &'a dyn Widget<Data = T>, data: &'a T) -> Self {
        // Safety: since the vtable for dyn Widget<Data = T> only uses T as &T
        // and T: Sized, the vtable should be equivalent for all T.
        // We ensure here that the type of `data` matches that used by `widget`.
        // NOTE: This makes assumptions beyond Rust's specification.
        use std::mem::transmute;
        unsafe { Node(transmute(widget), transmute(data)) }
    }

    /// Reborrow with a new lifetime
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    #[inline(always)]
    pub fn re<'b>(&'b self) -> Node<'b>
    where
        'a: 'b,
    {
        Node(self.0, self.1)
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

    /// Get the name of the widget struct
    #[inline]
    pub fn widget_name(&self) -> &'static str {
        self.0.widget_name()
    }

    /// Display as "StructName#WidgetId"
    #[inline]
    pub fn identify(&self) -> IdentifyWidget {
        IdentifyWidget(self.widget_name(), self.id_ref())
    }

    /// Get the number of child widgets
    ///
    /// Every value in the range `0..self.num_children()` is a valid child
    /// index.
    #[inline]
    pub fn num_children(&self) -> usize {
        self.0.num_children()
    }

    /// Get a child by index, if any
    ///
    /// Required: `index < self.num_children()`.
    #[inline]
    pub fn get_child(&self, index: usize) -> Option<Node<'_>> {
        self.0.get_child(self.1, index)
    }

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// If `Some(index)` is returned, this is *probably* but not guaranteed
    /// to be a valid child index.
    #[inline]
    pub fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
        id.next_key_after(self.id_ref())
    }

    /// Find the descendant with this `id`, if any
    ///
    /// Note: method consumes `self`, so in some cases you will need `node.re().find_node(id)`.
    pub fn find_node(self, id: &WidgetId) -> Option<Node<'a>> {
        if let Some(index) = self.find_child_index(id) {
            self.0
                .get_child(self.1, index)
                .and_then(|child| child.find_node(id))
        } else if self.eq_id(id) {
            Some(self)
        } else {
            None
        }
    }
}

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl<'a> Node<'a> {
    /// Get translation of children relative to this widget
    pub(crate) fn translation(&self) -> Offset {
        self.0.translation()
    }
}

/// Public API over a contextualized mutable widget
///
/// Note: this type has no publically supported utility over [`Node`].
/// It is, however, required for Kas's internals.
pub struct NodeMut<'a>(&'a mut dyn Widget<Data = ()>, &'a ());

impl<'a> NodeMut<'a> {
    /// Construct
    #[inline(always)]
    pub fn new<T: 'a>(widget: &'a mut dyn Widget<Data = T>, data: &'a T) -> Self {
        // Safety: since the vtable for dyn Widget<Data = T> only uses T as &T
        // and T: Sized, the vtable should be equivalent for all T.
        // We ensure here that the type of `data` matches that used by `widget`.
        // NOTE: This makes assumptions beyond Rust's specification.
        use std::mem::transmute;
        unsafe { NodeMut(transmute(widget), transmute(data)) }
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
        NodeMut(self.0, self.1)
    }

    /// Convert to non-mutable [`Node`]
    pub fn as_node(self) -> Node<'a> {
        Node(self.0, self.1)
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

    /// Get the name of the widget struct
    #[inline]
    pub fn widget_name(&self) -> &'static str {
        self.0.widget_name()
    }

    /// Display as "StructName#WidgetId"
    #[inline]
    pub fn identify(&self) -> IdentifyWidget {
        IdentifyWidget(self.widget_name(), self.id_ref())
    }

    /// Get the number of child widgets
    ///
    /// Every value in the range `0..self.num_children()` is a valid child
    /// index.
    #[inline]
    pub fn num_children(&self) -> usize {
        self.0.num_children()
    }

    /// Get a child mutably by index, if any
    ///
    /// Required: `index < self.num_children()`.
    #[inline]
    pub fn get_child(&mut self, index: usize) -> Option<NodeMut<'_>> {
        self.0.get_child_mut(self.1, index)
    }

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// If `Some(index)` is returned, this is *probably* but not guaranteed
    /// to be a valid child index.
    #[inline]
    pub fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
        id.next_key_after(self.id_ref())
    }

    /// Find the descendant with this `id`, if any
    ///
    /// Note: method consumes `self`, so in some cases you will need `node.re().find_node(id)`.
    pub fn find_node(self, id: &WidgetId) -> Option<NodeMut<'a>> {
        if let Some(index) = self.find_child_index(id) {
            self.0
                .get_child_mut(self.1, index)
                .and_then(|child| child.find_node(id))
        } else if self.eq_id(id) {
            Some(self)
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

    /// Translate a coordinate to a [`WidgetId`]
    pub(crate) fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        self.0.find_id(coord)
    }

    /// Draw a widget and its children
    pub(crate) fn _draw(&mut self, mut draw: DrawMgr) {
        draw.recurse(&mut self.0);
    }

    /// Internal method: configure recursively
    pub(crate) fn _configure(&mut self, cx: &mut ConfigMgr, id: WidgetId) {
        self.0._configure(self.1, cx, id);
    }

    /// Internal method: update recursively
    pub(crate) fn _update(&mut self, cx: &mut ConfigMgr) {
        self.0._update(self.1, cx);
    }

    /// Internal method: broadcast recursively
    pub(crate) fn _broadcast(&mut self, cx: &mut EventMgr, count: &mut usize, event: Event) {
        self.0._broadcast(self.1, cx, count, event);
    }

    /// Internal method: send recursively
    pub(crate) fn _send(
        &mut self,
        cx: &mut EventMgr,
        id: WidgetId,
        disabled: bool,
        event: Event,
    ) -> Response {
        self.0._send(self.1, cx, id, disabled, event)
    }

    /// Internal method: replay recursively
    pub(crate) fn _replay(&mut self, cx: &mut EventMgr, id: WidgetId, msg: Erased) {
        self.0._replay(self.1, cx, id, msg);
    }

    /// Internal method: search for the previous/next navigation target
    // NOTE: public on account of ListView
    pub fn _nav_next(
        &mut self,
        cx: &mut EventMgr,
        focus: Option<&WidgetId>,
        advance: NavAdvance,
    ) -> Option<WidgetId> {
        self.0._nav_next(self.1, cx, focus, advance)
    }
}
