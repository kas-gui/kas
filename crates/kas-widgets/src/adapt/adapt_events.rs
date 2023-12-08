// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event adapters

use super::{AdaptConfigCx, AdaptEventCx};
use kas::event::{ConfigCx, EventCx};
use kas::{autoimpl, impl_scope, widget_index, Events, LayoutExt, Widget};
use std::fmt::Debug;

impl_scope! {
    /// Wrapper to call a closure on update
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(Scrollable using self.inner where W: trait)]
    #[widget {
        layout = self.inner;
    }]
    pub struct OnUpdate<W: Widget> {
        core: widget_core!(),
        #[widget]
        pub inner: W,
        on_configure: Option<Box<dyn Fn(&mut AdaptConfigCx, &mut W)>>,
        on_update: Option<Box<dyn Fn(&mut AdaptConfigCx, &mut W, &W::Data)>>,
        message_handlers: Vec<Box<dyn Fn(&mut AdaptEventCx, &mut W, &W::Data)>>,
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new(inner: W) -> Self {
            OnUpdate {
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

    impl Events for Self {
        type Data = W::Data;

        // This is a little bit hacky: the closures operate on self.inner, so we
        // need to ensure that is configured / updated first!

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            let id = self.make_child_id(widget_index!(self.inner));
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
    }
}
