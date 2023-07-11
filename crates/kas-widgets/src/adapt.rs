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
        message_handler: Option<Box<dyn Fn(&mut EventMgr, &A, &mut S)>>,
        _data: PhantomData<A>,
    }

    impl Self {
        /// Construct over `inner` with additional `state`
        pub fn new(inner: W, state: S) -> Self {
            Adapt {
                core: Default::default(),
                state,
                inner,
                message_handler: None,
                _data: PhantomData,
            }
        }

        /// Add a generic message handler
        pub fn with_handler<H>(mut self, message_handler: H) -> Self
        where
            H: Fn(&mut EventMgr, &A, &mut S) + 'static,
        {
            debug_assert!(self.message_handler.is_none());
            self.message_handler = Some(Box::new(message_handler));
            self
        }

        /// Add a handler on message of type `M`
        pub fn on_message<M, H>(mut self, message_handler: H) -> Self
        where
            M: Debug + 'static,
            H: Fn(&A, &mut S, M) + 'static,
        {
            debug_assert!(self.message_handler.is_none());
            self.message_handler = Some(Box::new(move |mgr, data, state| {
                if let Some(m) = mgr.try_pop() {
                    message_handler(data, state, m);
                }
            }));
            self
        }
    }

    impl Events for Self {
        type Data = A;

        fn handle_message(&mut self, data: &Self::Data, mgr: &mut EventMgr) {
            if let Some(handler) = self.message_handler.as_ref() {
                handler(mgr, data, &mut self.state);
                mgr.update(self.as_node_mut(data));
            }
        }
    }
}
