// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Node

use super::Widget;
use crate::event::{ConfigMgr, Event, EventMgr, Response, Scroll};
use crate::geom::{Coord, Offset, Rect};
use crate::layout::{AxisInfo, SizeRules};
use crate::theme::{DrawMgr, SizeMgr};
use crate::util::IdentifyWidget;
use crate::WidgetId;

/// Public API over a contextualized widget
//
// NOTE: we unsafely transmute the data type of both the widget Data type and
// the data reference. Alternative: store a `Box<dyn NodeT>` where `NodeT` is
// a trait offering roughly this same API, implemented over a
// `struct NodeRef<'a, W: Widget>(&'a mut W, &'a W::Data);`.
pub struct Node<'a>(&'a mut dyn Widget<Data = ()>, &'a ());

impl<'a> Node<'a> {
    /// Construct
    // TODO: should this be hidden?
    pub fn new<T>(widget: &'a mut dyn Widget<Data = T>, data: &'a T) -> Self {
        use std::mem::transmute;
        let widget: &'a mut dyn Widget<Data = T> = widget;
        unsafe { Node(transmute(widget), transmute(data)) }
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

    /// Find the descendant with this `id`, if any
    pub fn find_widget(self, id: &WidgetId) -> Option<Node<'a>> {
        if let Some(index) = self.find_child_index(id) {
            self.get_child(index)
                .and_then(|child| child.find_widget(id))
        } else if self.eq_id(id) {
            return Some(self);
        } else {
            None
        }
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
        IdentifyWidget(self.widget_name(), self.id())
    }

    /// Get the number of child widgets
    ///
    /// Every value in the range `0..self.num_children()` is a valid child
    /// index.
    #[inline]
    pub fn num_children(&self) -> usize {
        self.0.num_children()
    }

    /// Get a child by index (if valid)
    ///
    /// Returns `Some(_)` exactly when `index < self.num_children()`.
    ///
    /// Warning: directly adjusting a widget without requiring reconfigure or
    /// redraw may break the UI. If a widget is replaced, a reconfigure **must**
    /// be requested. This can be done via [`EventState::send_action`].
    /// This method may be removed in the future.
    #[inline]
    pub fn get_child(self, index: usize) -> Option<Node<'a>> {
        self.0.get_child(self.1, index)
    }

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// If `Some(index)` is returned, this is *probably* but not guaranteed
    /// to be a valid child index.
    ///
    /// The default implementation simply uses [`WidgetId::next_key_after`].
    /// Widgets may choose to assign children custom keys by overriding this
    /// method and [`Self::make_child_id`].
    #[inline]
    pub fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
        id.next_key_after(self.id_ref())
    }

    /// Make an identifier for a child
    ///
    /// Default impl: `self.id_ref().make_child(index)`
    #[inline]
    pub fn make_child_id(&mut self, index: usize) -> WidgetId {
        self.id_ref().make_child(index)
    }

    /// Translate a coordinate to a [`WidgetId`]
    ///
    /// This method is used to determine which widget reacts to the mouse cursor
    /// or a touch event. The result affects mouse-hover highlighting, event
    /// handling by the target, and potentially also event handling by other
    /// widgets (e.g. a `Label` widget will not handle touch events, but if it
    /// is contained by a `ScrollRegion`, that widget may capture these via
    /// [`Widget::handle_unused`] to implement touch scrolling).
    ///
    /// The result is usually the widget which draws at the given `coord`, but
    /// does not have to be. For example, a `Button` widget will return its own
    /// `id` for coordinates drawn by internal content, while the `CheckButton`
    /// widget uses an internal component for event handling and thus reports
    /// this component's `id` even over its own area.
    #[inline]
    pub fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        self.0.find_id(coord)
    }
}

impl<'a> Node<'a> {
    /// Get size rules for the given axis
    #[inline]
    pub(crate) fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        self.0.size_rules(size_mgr, axis)
    }

    /// Set size and position
    #[inline]
    pub(crate) fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
        self.0.set_rect(mgr, rect);
    }

    /// Draw a widget and its children
    #[inline]
    pub(crate) fn draw(&mut self, draw: DrawMgr) {
        self.0.draw(draw);
    }

    /// Pre-configuration
    #[inline]
    pub(crate) fn pre_configure(&mut self, mgr: &mut ConfigMgr, id: WidgetId) {
        self.0.pre_configure(&mut mgr.with_data(self.1), id);
    }

    /// Configure widget
    #[inline]
    pub(crate) fn configure(&mut self, mgr: &mut ConfigMgr) {
        self.0.configure(&mut mgr.with_data(self.1));
    }

    /// Is this widget navigable via <kbd>Tab</kbd> key?
    #[inline]
    pub(crate) fn navigable(&self) -> bool {
        self.0.navigable()
    }

    /// Get translation of children relative to this widget
    #[inline]
    pub(crate) fn translation(&self) -> Offset {
        self.0.translation()
    }

    /// Navigation in spatial order
    #[inline]
    pub(crate) fn nav_next(
        &mut self,
        mgr: &mut EventMgr,
        reverse: bool,
        from: Option<usize>,
    ) -> Option<usize> {
        self.0.nav_next(&mut mgr.with_data(self.1), reverse, from)
    }

    /// Pre-event-handler
    #[inline]
    pub(crate) fn pre_handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
        self.0.pre_handle_event(&mut mgr.with_data(self.1), event)
    }

    /// Handle an [`Event`] sent to this widget
    #[inline]
    pub(crate) fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
        self.0.handle_event(&mut mgr.with_data(self.1), event)
    }

    /// Potentially steal an event before it reaches a child
    #[inline]
    pub(crate) fn steal_event(
        &mut self,
        mgr: &mut EventMgr,
        id: &WidgetId,
        event: &Event,
    ) -> Response {
        self.0.steal_event(&mut mgr.with_data(self.1), id, event)
    }

    /// Handle an event sent to child `index` but left unhandled
    #[inline]
    pub(crate) fn handle_unused(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
        self.0.handle_unused(&mut mgr.with_data(self.1), event)
    }

    /// Handler for messages from children/descendants
    #[inline]
    pub(crate) fn handle_message(&mut self, mgr: &mut EventMgr) {
        self.0.handle_message(&mut mgr.with_data(self.1));
    }

    /// Handler for scrolling
    #[inline]
    pub(crate) fn handle_scroll(&mut self, mgr: &mut EventMgr, scroll: Scroll) {
        self.0.handle_scroll(&mut mgr.with_data(self.1), scroll);
    }
}
