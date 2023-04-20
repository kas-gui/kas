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
    pub struct Adapt<A, W: Widget, S: Debug, F>
    where
        F: for<'a> Fn(&'a A, &'a S) -> &'a W::Data,
    {
        core: widget_core!(),
        state: S,
        #[widget((self.map_fn)(data, &self.state))]
        inner: W,
        map_fn: F,
        message_handler: Option<Box<dyn Fn(&mut EventMgr, &A, &mut S)>>,
        _data: PhantomData<A>,
    }

    impl Self {
        /// Construct
        ///
        /// -   Over an `inner` widget
        /// -   With additional `state`
        /// -   And `map_fn` mapping to the inner widget's data type
        pub fn new(inner: W, state: S, map_fn: F) -> Self {
            Adapt {
                core: Default::default(),
                state,
                inner,
                map_fn,
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
            }
        }
    }
}
