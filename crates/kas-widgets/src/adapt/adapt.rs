// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapt widget

use kas::prelude::*;
use linear_map::LinearMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;

/// An [`EventCx`] with embedded [`Id`]
///
/// NOTE: this is a temporary design: it may be expanded or integrated with
/// `EventCx` in the future.
#[autoimpl(Deref, DerefMut using self.cx)]
pub struct AdaptEventCx<'a: 'b, 'b, A> {
    cx: &'b mut EventCx<'a>,
    id: Id,
    data: &'b A,
}

impl<'a: 'b, 'b, A> AdaptEventCx<'a, 'b, A> {
    #[inline]
    fn new(cx: &'b mut EventCx<'a>, id: Id, data: &'b A) -> Self {
        AdaptEventCx { cx, id, data }
    }

    /// Access input data
    #[inline]
    pub fn data(&'b self) -> &'b A {
        self.data
    }

    /// Check whether this widget is disabled
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.cx.is_disabled(&self.id)
    }

    /// Set/unset disabled status for this widget
    #[inline]
    pub fn set_disabled(&mut self, state: bool) {
        self.cx.set_disabled(self.id.clone(), state);
    }

    /// Schedule a timed update
    ///
    /// This widget will receive an update for timer `timer_id` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid except from a timer
    /// handler where it might cause an infinite loop.
    ///
    /// Multiple timer requests with the same `timer_id` are merged
    /// (choosing the earliest time).
    #[inline]
    pub fn request_timer(&mut self, timer_id: u64, delay: Duration) {
        self.cx.request_timer(self.id.clone(), timer_id, delay);
    }
}

/// A [`ConfigCx`] with embedded [`Id`]
///
/// NOTE: this is a temporary design: it may be expanded or integrated with
/// `ConfigCx` in the future.
#[autoimpl(Deref, DerefMut using self.cx)]
pub struct AdaptConfigCx<'a: 'b, 'b> {
    cx: &'b mut ConfigCx<'a>,
    id: Id,
}

impl<'a: 'b, 'b> AdaptConfigCx<'a, 'b> {
    #[inline]
    fn new(cx: &'b mut ConfigCx<'a>, id: Id) -> Self {
        AdaptConfigCx { cx, id }
    }

    /// Check whether this widget is disabled
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.cx.is_disabled(&self.id)
    }

    /// Set/unset disabled status for this widget
    #[inline]
    pub fn set_disabled(&mut self, state: bool) {
        self.cx.set_disabled(self.id.clone(), state);
    }

    /// Enable `alt_bypass` for layer
    ///
    /// This may be called by a child widget during configure to enable or
    /// disable alt-bypass for the access-key layer containing its access keys.
    /// This allows access keys to be used as shortcuts without the Alt
    /// key held. See also [`EventState::new_access_layer`].
    #[inline]
    pub fn enable_alt_bypass(&mut self, alt_bypass: bool) {
        self.cx.enable_alt_bypass(&self.id, alt_bypass);
    }

    /// Schedule a timed update
    ///
    /// This widget will receive an update for timer `timer_id` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid except from a timer
    /// handler where it might cause an infinite loop.
    ///
    /// Multiple timer requests with the same `timer_id` are merged
    /// (choosing the earliest time).
    #[inline]
    pub fn request_timer(&mut self, timer_id: u64, delay: Duration) {
        self.cx.request_timer(self.id.clone(), timer_id, delay);
    }
}

impl_scope! {
    /// Data adaption node
    ///
    /// Where [`Map`] allows mapping to a sub-set of input data, `Adapt` allows
    /// mapping to a super-set (including internal storage). Further, `Adapt`
    /// supports message handlers which mutate internal storage.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(Scrollable using self.inner where W: trait)]
    #[widget {
        layout = self.inner;
    }]
    pub struct Adapt<A, W: Widget<Data = S>, S: Debug> {
        core: widget_core!(),
        state: S,
        #[widget(&self.state)]
        inner: W,
        configure_handler: Option<Box<dyn Fn(&mut AdaptConfigCx, &mut S)>>,
        update_handler: Option<Box<dyn Fn(&mut AdaptConfigCx, &A, &mut S)>>,
        timer_handlers: LinearMap<u64, Box<dyn Fn(&mut AdaptEventCx<A>, &mut S) -> bool>>,
        message_handlers: Vec<Box<dyn Fn(&mut AdaptEventCx<A>, &mut S) -> bool>>,
    }

    impl Self {
        /// Construct over `inner` with additional `state`
        #[inline]
        pub fn new(inner: W, state: S) -> Self {
            Adapt {
                core: Default::default(),
                state,
                inner,
                configure_handler: None,
                update_handler: None,
                timer_handlers: LinearMap::new(),
                message_handlers: vec![],
            }
        }

        /// Add a handler to be called on configuration
        pub fn on_configure<F>(mut self, handler: F) -> Self
        where
            F: Fn(&mut AdaptConfigCx, &mut S) + 'static,
        {
            debug_assert!(self.configure_handler.is_none());
            self.configure_handler = Some(Box::new(handler));
            self
        }

        /// Add a handler to be called on update of input data
        ///
        /// Children will be updated after the handler is called.
        pub fn on_update<F>(mut self, handler: F) -> Self
        where
            F: Fn(&mut AdaptConfigCx, &A, &mut S) + 'static,
        {
            debug_assert!(self.update_handler.is_none());
            self.update_handler = Some(Box::new(handler));
            self
        }

        /// Set a timer handler
        ///
        /// The closure should return `true` if state was updated.
        pub fn on_timer<H>(mut self, timer_id: u64, handler: H) -> Self
        where
            H: Fn(&mut AdaptEventCx<A>, &mut S) -> bool + 'static,
        {
            debug_assert!(self.timer_handlers.get(&timer_id).is_none());
            self.timer_handlers.insert(timer_id, Box::new(handler));
            self
        }

        /// Add a handler on message of type `M`
        ///
        /// Children will be updated whenever this handler is invoked.
        ///
        /// Where multiple message types must be handled or access to the
        /// [`AdaptEventCx`] is required, use [`Self::on_messages`] instead.
        pub fn on_message<M, H>(self, handler: H) -> Self
        where
            M: Debug + 'static,
            H: Fn(&mut AdaptEventCx<A>, &mut S, M) + 'static,
        {
            self.on_messages(move |cx, state| {
                if let Some(m) = cx.try_pop() {
                    handler(cx, state, m);
                    true
                } else {
                    false
                }
            })
        }

        /// Add a generic message handler
        ///
        /// The closure should return `true` if state was updated.
        pub fn on_messages<H>(mut self, handler: H) -> Self
        where
            H: Fn(&mut AdaptEventCx<A>, &mut S) -> bool + 'static,
        {
            self.message_handlers.push(Box::new(handler));
            self
        }
    }

    impl Events for Self {
        type Data = A;

        fn configure(&mut self, cx: &mut ConfigCx) {
            if let Some(handler) = self.configure_handler.as_ref() {
                let mut cx = AdaptConfigCx::new(cx, self.id());
                handler(&mut cx, &mut self.state);
            }
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            if let Some(handler) = self.update_handler.as_ref() {
                let mut cx = AdaptConfigCx::new(cx, self.id());
                handler(&mut cx, data, &mut self.state);
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Timer(timer_id) => {
                    if let Some(handler) = self.timer_handlers.get(&timer_id) {
                        let mut cx = AdaptEventCx::new(cx, self.id(), data);
                        if handler(&mut cx, &mut self.state) {
                            cx.update(self.as_node(data));
                        }
                        Used
                    } else {
                        Unused
                    }
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &A) {
            let mut update = false;
            let mut cx = AdaptEventCx::new(cx, self.id(), data);
            for handler in self.message_handlers.iter() {
                update |= handler(&mut cx, &mut self.state);
            }
            if update {
                cx.update(self.as_node(data));
            }
        }
    }
}

impl_scope! {
    /// Data mapping
    ///
    /// This is a generic data-mapping widget. See also [`Adapt`], [`MapAny`](super::MapAny).
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(Scrollable using self.inner where W: trait)]
    #[widget {
        Data = A;
        layout = self.inner;
    }]
    pub struct Map<A, W: Widget, F>
    where
        F: for<'a> Fn(&'a A) -> &'a W::Data,
    {
        core: widget_core!(),
        #[widget((self.map_fn)(data))]
        inner: W,
        map_fn: F,
        _data: PhantomData<A>,
    }

    impl Self {
        /// Construct
        ///
        /// -   Over an `inner` widget
        /// -   And `map_fn` mapping to the inner widget's data type
        pub fn new(inner: W, map_fn: F) -> Self {
            Map {
                core: Default::default(),
                inner,
                map_fn,
                _data: PhantomData,
            }
        }
    }
}
