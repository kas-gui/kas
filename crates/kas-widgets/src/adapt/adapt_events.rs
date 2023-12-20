// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event adapters

use super::{AdaptConfigCx, AdaptEventCx};
use kas::autoimpl;
use kas::event::{ConfigCx, Event, EventCx, IsUsed};
use kas::geom::{Coord, Offset, Rect};
use kas::layout::{AxisInfo, SizeRules};
use kas::theme::{DrawCx, SizeCx};
#[allow(unused)] use kas::Events;
use kas::{Id, Layout, LayoutExt, NavAdvance, Node, Widget};
use std::fmt::Debug;

/// Wrapper with configure / update / message handling callbacks.
///
/// This type is constructed by some [`AdaptWidget`](super::AdaptWidget) methods.
#[autoimpl(Deref, DerefMut using self.inner)]
#[autoimpl(Scrollable using self.inner where W: trait)]
pub struct AdaptEvents<W: Widget> {
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
    /// The child index may be inferred via [`EventCx::last_child`].
    /// (Note: this is only possible since `AdaptEvents` is a special "thin" wrapper.)
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

    /// Add a child handler to map messages of type `M` to `N`
    ///
    /// # Example
    ///
    /// ```
    /// use kas::messages::Select;
    /// use kas_widgets::{AdaptWidget, Row, Tab};
    ///
    /// #[derive(Clone, Debug)]
    /// struct MsgSelectIndex(usize);
    ///
    /// let tabs = Row::new([Tab::new("A")])
    ///     .map_message(|index, Select| MsgSelectIndex(index));
    /// ```
    pub fn map_message<M, N, H>(self, handler: H) -> Self
    where
        M: Debug + 'static,
        N: Debug + 'static,
        H: Fn(usize, M) -> N + 'static,
    {
        self.on_messages(move |cx, _, _| {
            if let Some(index) = cx.last_child() {
                if let Some(m) = cx.try_pop() {
                    cx.push(handler(index, m));
                }
            }
        })
    }

    /// Add a generic message handler
    ///
    /// The child index may be inferred via [`EventCx::last_child`].
    /// (Note: this is only possible since `AdaptEvents` is a special "thin" wrapper.)
    #[must_use]
    pub fn on_messages<H>(mut self, handler: H) -> Self
    where
        H: Fn(&mut AdaptEventCx, &mut W, &W::Data) + 'static,
    {
        self.message_handlers.push(Box::new(handler));
        self
    }
}

impl<W: Widget> Widget for AdaptEvents<W> {
    type Data = W::Data;

    #[inline]
    fn as_node<'a>(&'a mut self, data: &'a Self::Data) -> Node<'a> {
        Node::new(self, data)
    }

    #[inline]
    fn for_child_node(
        &mut self,
        data: &Self::Data,
        index: usize,
        closure: Box<dyn FnOnce(Node<'_>) + '_>,
    ) {
        self.inner.for_child_node(data, index, closure);
    }

    #[inline]
    fn _configure(&mut self, cx: &mut ConfigCx, data: &Self::Data, id: Id) {
        self.inner._configure(cx, data, id);

        if let Some(ref f) = self.on_configure {
            let mut cx = AdaptConfigCx::new(cx, self.inner.id());
            f(&mut cx, &mut self.inner);
        }
        if let Some(ref f) = self.on_update {
            let mut cx = AdaptConfigCx::new(cx, self.inner.id());
            f(&mut cx, &mut self.inner, data);
        }
    }

    fn _update(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        self.inner._update(cx, data);

        if let Some(ref f) = self.on_update {
            let mut cx = AdaptConfigCx::new(cx, self.inner.id());
            f(&mut cx, &mut self.inner, data);
        }
    }

    #[inline]
    fn _send(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id, event: Event) -> IsUsed {
        let is_used = self.inner._send(cx, data, id, event);

        if cx.has_msg() {
            let mut cx = AdaptEventCx::new(cx, self.inner.id());
            for handler in self.message_handlers.iter() {
                handler(&mut cx, &mut self.inner, data);
            }
        }

        is_used
    }

    #[inline]
    fn _replay(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id) {
        self.inner._replay(cx, data, id);

        if cx.has_msg() {
            let mut cx = AdaptEventCx::new(cx, self.inner.id());
            for handler in self.message_handlers.iter() {
                handler(&mut cx, &mut self.inner, data);
            }
        }
    }

    #[inline]
    fn _nav_next(
        &mut self,
        cx: &mut ConfigCx,
        data: &Self::Data,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id> {
        self.inner._nav_next(cx, data, focus, advance)
    }
}

impl<W: Widget> Layout for AdaptEvents<W> {
    #[inline]
    fn as_layout(&self) -> &dyn Layout {
        self
    }

    #[inline]
    fn id_ref(&self) -> &Id {
        self.inner.id_ref()
    }

    #[inline]
    fn rect(&self) -> Rect {
        self.inner.rect()
    }

    #[inline]
    fn widget_name(&self) -> &'static str {
        "AdaptEvents"
    }

    #[inline]
    fn num_children(&self) -> usize {
        self.inner.num_children()
    }

    #[inline]
    fn get_child(&self, index: usize) -> Option<&dyn Layout> {
        self.inner.get_child(index)
    }

    #[inline]
    fn find_child_index(&self, id: &Id) -> Option<usize> {
        self.inner.find_child_index(id)
    }

    #[inline]
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.inner.size_rules(sizer, axis)
    }

    #[inline]
    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        self.inner.set_rect(cx, rect);
    }

    #[inline]
    fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        self.inner.nav_next(reverse, from)
    }

    #[inline]
    fn translation(&self) -> Offset {
        self.inner.translation()
    }

    #[inline]
    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        self.inner.find_id(coord)
    }

    #[inline]
    fn draw(&mut self, draw: DrawCx) {
        self.inner.draw(draw);
    }
}
