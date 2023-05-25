// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapt widget
//!
//! TODO: add Hash widget which requires A: Hash, calculates the hash on each
//! update, and does not recurse update calls if the hash matches.
//! Maybe also Clone variant. And Discard never needs to recurse update.
//! Not currently possible since there is no control over recursion of update.

use kas::prelude::*;
use std::fmt::Debug;
use std::marker::PhantomData;

impl_scope! {
    /// Data adaption node
    ///
    /// Where [`Map`] allows mapping to a sub-set of input data, `Adapt` allows
    /// mapping to a super-set (including internal storage). Further, `Adapt`
    /// supports message handlers which mutate internal storage.
    #[autoimpl(Debug ignore self.map_fn, self.message_handlers, self._data)]
    #[autoimpl(Deref, DerefMut using self.inner)]
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
        fn handle_messages(&mut self, cx: &mut EventCx<Self::Data>) {
            for mh in self.message_handlers.iter() {
                mh(cx, &mut self.state);
            }

            cx.config_cx(|cx| cx.update(self));
        }
    }
}

impl_scope! {
    /// Data mapping
    ///
    /// This is a generic data-mapping widget. See also [`Discard`], [`Adapt`].
    #[autoimpl(Debug ignore self.map_fn, self._data)]
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[widget {
        data = A;
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
