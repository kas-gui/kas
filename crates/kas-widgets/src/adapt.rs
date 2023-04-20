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
    #[autoimpl(Debug ignore self.map_fn, self.message_handlers, self._data)]
    #[widget {
        data = A;
        layout = self.inner;
    }]
    pub struct Adapt<A, W: Widget, S: Debug, F>
    where
        F: for<'a> Fn(&'a A, &'a S) -> &'a W::Data,
    {
        // TODO: do we need a core?
        core: widget_core!(),
        state: S,
        #[widget((self.map_fn)(data, &self.state))]
        inner: W,
        map_fn: F,
        message_handlers: Vec<Box<dyn Fn(&mut EventCx<A>, &mut S)>>,
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
                message_handlers: vec![],
                _data: PhantomData,
            }
        }

        /// Add a generic message handler
        pub fn with_handler<H>(mut self, message_handler: H) -> Self
        where
            H: Fn(&mut EventCx<A>, &mut S) + 'static,
        {
            self.message_handlers.push(Box::new(message_handler));
            self
        }

        /// Add a handler on message of type `M`
        pub fn on_message<M, H>(mut self, message_handler: H) -> Self
        where
            M: Debug + 'static,
            H: Fn(&A, &mut S, M) + 'static,
        {
            self.message_handlers.push(Box::new(move |cx, data| {
                if let Some(m) = cx.try_pop() {
                    message_handler(cx.data(), data, m);
                }
            }));
            self
        }
    }

    impl Widget for Self {
        fn update(&mut self, cx: &mut ConfigCx<A>) {
            self.inner.update(&mut cx.map(|data| (self.map_fn)(data, &self.state)));
        }
    }
}

impl_scope! {
    /// Data adaptation: map to ()
    #[widget {
        data = A;
        layout = self.inner;
    }]
    #[autoimpl(Debug)]
    pub struct DiscardData<A, W: Widget<Data = ()>> {
        core: widget_core!(),
        #[widget(&())]
        inner: W,
        _data: PhantomData<A>,
    }

    impl Self {
        /// Construct
        pub fn new(inner: W) -> Self {
            DiscardData {
                core: Default::default(),
                inner,
                _data: PhantomData,
            }
        }
    }
}
