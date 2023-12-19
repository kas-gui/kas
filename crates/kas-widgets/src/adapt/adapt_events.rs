// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event adapters

use super::{AdaptConfigCx, AdaptEventCx};
use kas::event::{ConfigCx, Event, EventCx, IsUsed, Unused};
use kas::geom::{Coord, Rect};
use kas::layout::{AxisInfo, SizeRules, Visitor};
use kas::theme::{DrawCx, SizeCx};
use kas::{autoimpl, widget_index};
use kas::{CoreData, Events, Id, Layout, LayoutExt, NavAdvance, Node, Widget};
use std::fmt::Debug;

/// Wrapper with configure / update / message handling callbacks.
///
/// This type is constructed by some [`AdaptWidget`](super::AdaptWidget) methods.
#[autoimpl(Deref, DerefMut using self.inner)]
#[autoimpl(Scrollable using self.inner where W: trait)]
pub struct AdaptEvents<W: Widget> {
    core: CoreData,
    pub inner: W,
    on_configure: Option<Box<dyn Fn(&mut AdaptConfigCx, &mut W)>>,
    on_update: Option<Box<dyn Fn(&mut AdaptConfigCx, &mut W, &W::Data)>>,
    message_handlers: Vec<Box<dyn Fn(&mut AdaptEventCx, &mut W, &W::Data)>>,
}

impl<W: Widget> AdaptEvents<W> {
    /// Construct
    #[inline]
    pub fn new(inner: W) -> Self {
        AdaptEvents {
            core: Default::default(),
            inner,
            on_configure: None,
            on_update: None,
            message_handlers: vec![],
        }
    }

    /// Call the given closure on [`Events::configure`]
    #[must_use]
    pub fn on_configure<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut AdaptConfigCx, &mut W) + 'static,
    {
        self.on_configure = Some(Box::new(f));
        self
    }

    /// Call the given closure on [`Events::update`]
    #[must_use]
    pub fn on_update<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut AdaptConfigCx, &mut W, &W::Data) + 'static,
    {
        self.on_update = Some(Box::new(f));
        self
    }

    /// Add a handler on message of type `M`
    ///
    /// Where access to input data is required, use [`Self::on_messages`] instead.
    #[must_use]
    pub fn on_message<M, H>(self, handler: H) -> Self
    where
        M: Debug + 'static,
        H: Fn(&mut AdaptEventCx, &mut W, M) + 'static,
    {
        self.on_messages(move |cx, w, _data| {
            if let Some(m) = cx.try_pop() {
                handler(cx, w, m);
            }
        })
    }

    /// Add a generic message handler
    #[must_use]
    pub fn on_messages<H>(mut self, handler: H) -> Self
    where
        H: Fn(&mut AdaptEventCx, &mut W, &W::Data) + 'static,
    {
        self.message_handlers.push(Box::new(handler));
        self
    }
}

impl<W: Widget> Events for AdaptEvents<W> {
    fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        let id = self.make_child_id(widget_index!(0usize));
        cx.configure(self.inner.as_node(data), id.clone());
        if let Some(ref f) = self.on_configure {
            let mut cx = AdaptConfigCx::new(cx, id.clone());
            f(&mut cx, &mut self.inner);
        }
        if let Some(ref f) = self.on_update {
            let mut cx = AdaptConfigCx::new(cx, id);
            f(&mut cx, &mut self.inner, data);
        }
    }

    fn update_recurse(&mut self, cx: &mut ConfigCx, data: &W::Data) {
        cx.update(self.inner.as_node(data));
        if let Some(ref f) = self.on_update {
            let mut cx = AdaptConfigCx::new(cx, self.inner.id());
            f(&mut cx, &mut self.inner, data);
        }
    }

    fn handle_messages(&mut self, cx: &mut EventCx, data: &W::Data) {
        let mut cx = AdaptEventCx::new(cx, self.inner.id());
        for handler in self.message_handlers.iter() {
            handler(&mut cx, &mut self.inner, data);
        }
    }

    fn steal_event(&mut self, _: &mut EventCx, _: &Self::Data, _: &Id, _: &Event) -> IsUsed {
        #[cfg(debug_assertions)]
        self.core.status.require_rect(&self.core.id);
        Unused
    }

    fn handle_event(&mut self, _: &mut EventCx, _: &Self::Data, _: Event) -> IsUsed {
        #[cfg(debug_assertions)]
        self.core.status.require_rect(&self.core.id);
        Unused
    }
}
impl<W: Widget> Widget for AdaptEvents<W> {
    type Data = W::Data;

    #[inline]
    fn as_node<'a>(&'a mut self, data: &'a Self::Data) -> Node<'a> {
        Node::new(self, data)
    }

    fn for_child_node(
        &mut self,
        data: &Self::Data,
        index: usize,
        closure: Box<dyn FnOnce(Node<'_>) + '_>,
    ) {
        match index {
            0usize => closure(self.inner.as_node(data)),
            _ => (),
        }
    }

    fn _configure(&mut self, cx: &mut ConfigCx, data: &Self::Data, id: Id) {
        self.core.id = id;
        #[cfg(debug_assertions)]
        self.core.status.configure(&self.core.id);
        Events::configure(self, cx);
        Events::update(self, cx, data);
        Events::configure_recurse(self, cx, data);
    }

    fn _update(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        #[cfg(debug_assertions)]
        self.core.status.update(&self.core.id);
        Events::update(self, cx, data);
        Events::update_recurse(self, cx, data);
    }

    fn _send(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id, event: Event) -> IsUsed {
        kas::impls::_send(self, cx, data, id, event)
    }

    fn _replay(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id) {
        kas::impls::_replay(self, cx, data, id);
    }

    fn _nav_next(
        &mut self,
        cx: &mut ConfigCx,
        data: &Self::Data,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id> {
        kas::impls::_nav_next(self, cx, data, focus, advance)
    }
}

impl<W: Widget> Layout for AdaptEvents<W> {
    #[inline]
    fn as_layout(&self) -> &dyn Layout {
        self
    }

    #[inline]
    fn id_ref(&self) -> &Id {
        &self.core.id
    }

    #[inline]
    fn rect(&self) -> Rect {
        self.core.rect
    }

    #[inline]
    fn widget_name(&self) -> &'static str {
        "AdaptEvents"
    }

    fn num_children(&self) -> usize {
        1usize
    }

    fn get_child(&self, index: usize) -> Option<&dyn Layout> {
        match index {
            0usize => Some(self.inner.as_layout()),
            _ => None,
        }
    }

    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        #[cfg(debug_assertions)]
        self.core.status.size_rules(&self.core.id, axis);

        (Visitor::single(&mut self.inner)).size_rules(sizer, axis)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        #[cfg(debug_assertions)]
        self.core.status.set_rect(&self.core.id);

        self.core.rect = rect;
        (Visitor::single(&mut self.inner)).set_rect(cx, rect);
    }

    fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        let mut iter = [0usize].into_iter();
        if !reverse {
            if let Some(wi) = from {
                let _ = iter.find(|x| *x == wi);
            }
            iter.next()
        } else {
            let mut iter = iter.rev();
            if let Some(wi) = from {
                let _ = iter.find(|x| *x == wi);
            }
            iter.next()
        }
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        #[cfg(debug_assertions)]
        self.core.status.require_rect(&self.core.id);

        if !self.rect().contains(coord) {
            return None;
        }

        let coord = coord + self.translation();
        (Visitor::single(&mut self.inner))
            .find_id(coord)
            .or_else(|| Some(self.id()))
    }

    fn draw(&mut self, draw: DrawCx) {
        #[cfg(debug_assertions)]
        self.core.status.require_rect(&self.core.id);

        (Visitor::single(&mut self.inner)).draw(draw);
    }
}
