// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapt widget

use super::{AdaptConfigCx, AdaptEventCx};
use kas::prelude::*;
use linear_map::LinearMap;
use std::fmt::Debug;
use std::marker::PhantomData;

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
        update_handler: Option<Box<dyn Fn(&mut AdaptConfigCx, &mut S, &A)>>,
        timer_handlers: LinearMap<u64, Box<dyn Fn(&mut AdaptEventCx, &mut S, &A)>>,
        message_handlers: Vec<Box<dyn Fn(&mut AdaptEventCx, &mut S, &A)>>,
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
            F: Fn(&mut AdaptConfigCx, &mut S, &A) + 'static,
        {
            debug_assert!(self.update_handler.is_none());
            self.update_handler = Some(Box::new(handler));
            self
        }

        /// Set a timer handler
        ///
        /// It is assumed that state is modified by this timer. Frequent usage
        /// of timers which don't do anything may be inefficient; prefer usage
        /// of [`EventState::push_async`](kas::event::EventState::push_async).
        pub fn on_timer<H>(mut self, timer_id: u64, handler: H) -> Self
        where
            H: Fn(&mut AdaptEventCx, &mut S, &A) + 'static,
        {
            debug_assert!(self.timer_handlers.get(&timer_id).is_none());
            self.timer_handlers.insert(timer_id, Box::new(handler));
            self
        }

        /// Add a handler on message of type `M`
        ///
        /// Children will be updated whenever this handler is invoked.
        ///
        /// Where access to input data (from parent widgets) is required,
        /// use [`Self::on_messages`] instead.
        pub fn on_message<M, H>(self, handler: H) -> Self
        where
            M: Debug + 'static,
            H: Fn(&mut AdaptEventCx, &mut S, M) + 'static,
        {
            self.on_messages(move |cx, state, _data| {
                if let Some(m) = cx.try_pop() {
                    handler(cx, state, m);
                }
            })
        }

        /// Add a generic message handler
        pub fn on_messages<H>(mut self, handler: H) -> Self
        where
            H: Fn(&mut AdaptEventCx, &mut S, &A) + 'static,
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
                handler(&mut cx, &mut self.state, data);
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Timer(timer_id) => {
                    if let Some(handler) = self.timer_handlers.get(&timer_id) {
                        let mut cx = AdaptEventCx::new(cx, self.id());
                        handler(&mut cx, &mut self.state, data);
                        cx.update(self.as_node(data));
                        Used
                    } else {
                        Unused
                    }
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &A) {
            let count = cx.msg_op_count();
            let mut cx = AdaptEventCx::new(cx, self.id());
            for handler in self.message_handlers.iter() {
                handler(&mut cx, &mut self.state, data);
            }
            if cx.msg_op_count() != count {
                cx.update(self.as_node(data));
            }
        }
    }
}

impl_scope! {
    /// Data mapping
    ///
    /// This is a generic data-mapping widget-wrapper.
    /// See also [`Adapt`], [`MapAny`](super::MapAny).
    ///
    /// This struct is a thin wrapper around the inner widget without its own
    /// [`Id`]. It supports [`Deref`](std::ops::Deref) and
    /// [`DerefMut`](std::ops::DerefMut) to the inner widget.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(Scrollable using self.inner where W: trait)]
    #[widget {
        Data = A;
        data_expr = (self.map_fn)(data);
        derive = self.inner;
    }]
    pub struct Map<A, W: Widget, F>
    where
        F: for<'a> Fn(&'a A) -> &'a W::Data,
    {
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
                inner,
                map_fn,
                _data: PhantomData,
            }
        }
    }
}
