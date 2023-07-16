// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapt widget

use kas::prelude::*;
use std::fmt::Debug;
use std::marker::PhantomData;

impl_scope! {
    /// Data adaption node
    ///
    /// This node adapts an input data type to some output type with additional
    /// state. It may also handle messages to update its data.
    #[widget {
        layout = self.inner;
    }]
    pub struct Adapt<A, W: Widget<Data = S>, S: Debug> {
        core: widget_core!(),
        state: S,
        #[widget(&self.state)]
        inner: W,
        need_update: bool,
        message_handler: Option<Box<dyn Fn(&mut EventMgr, &A, &mut S) -> bool>>,
        update_handler: Option<Box<dyn Fn(&mut ConfigMgr, &A, &mut S)>>,
        _data: PhantomData<A>,
    }

    impl Self {
        /// Construct over `inner` with additional `state`
        pub fn new(inner: W, state: S) -> Self {
            Adapt {
                core: Default::default(),
                state,
                inner,
                need_update: false,
                message_handler: None,
                update_handler: None,
                _data: PhantomData,
            }
        }

        /// Add a handler on message of type `M`
        ///
        /// Children will be updated whenever this handler is invoked.
        ///
        /// Where multiple message types must be handled or access to the
        /// [`EventMgr`] is required, use [`Self::on_messages`] instead.
        pub fn on_message<M, H>(self, message_handler: H) -> Self
        where
            M: Debug + 'static,
            H: Fn(&A, &mut S, M) + 'static,
        {
            self.on_messages(move |mgr, data, state| {
                if let Some(m) = mgr.try_pop() {
                    message_handler(data, state, m);
                    true
                } else {
                    false
                }
            })
        }

        /// Add a generic message handler
        ///
        /// Children will be updated if this handler returns `true`.
        pub fn on_messages<H>(mut self, message_handler: H) -> Self
        where
            H: Fn(&mut EventMgr, &A, &mut S) -> bool + 'static,
        {
            debug_assert!(self.message_handler.is_none());
            self.message_handler = Some(Box::new(message_handler));
            self
        }

        /// Add a handler to be called on update of input data
        ///
        /// Children will be updated after the handler is called.
        pub fn on_update<F>(mut self, update_handler: F) -> Self
        where
            F: Fn(&mut ConfigMgr, &A, &mut S) + 'static,
        {
            debug_assert!(self.update_handler.is_none());
            self.update_handler = Some(Box::new(update_handler));
            self
        }
    }

    impl Events for Self {
        type Data = A;

        fn update(&mut self, data: &A, cx: &mut ConfigMgr) {
            if let Some(handler) = self.update_handler.as_ref() {
                handler(cx, data, &mut self.state);
            } else if self.need_update {
                self.need_update = false;
            } else {
                cx.inhibit_recursion();
            }
        }

        fn handle_messages(&mut self, data: &A, mgr: &mut EventMgr) {
            if let Some(handler) = self.message_handler.as_ref() {
                if handler(mgr, data, &mut self.state) {
                    self.need_update = true;
                    mgr.update(self.as_node_mut(data));
                }
            }
        }
    }
}
