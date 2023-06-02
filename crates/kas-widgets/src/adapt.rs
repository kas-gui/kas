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
    #[autoimpl(Debug ignore self.map_fn, self.on_messages, self.on_update)]
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
        on_messages: Option<Box<dyn Fn(&mut EventCx<A>, &mut S)>>,
        on_update: Option<Box<dyn Fn(&mut ConfigCx<A>, &mut S)>>,
    }

    impl Self {
        /// Construct
        ///
        /// -   Over an `inner` widget
        /// -   With additional `state`
        /// -   And `map_fn` mapping to the inner widget's data type
        #[inline]
        pub fn new(inner: W, state: S, map_fn: F) -> Self {
            Adapt {
                core: Default::default(),
                state,
                inner,
                map_fn,
                on_messages: None,
                on_update: None,
            }
        }

        /// Assign a handler for a message of type `M`
        ///
        /// This is a variant of [`Self::on_messages`] for ease of use.
        /// Parameters: `(data, state, message)` where `data` is input from the
        /// parent (read-only) while `state` is stored locally (read-write).
        pub fn on_message<M, H>(self, f: H) -> Self
        where
            M: Debug + 'static,
            H: Fn(&A, &mut S, M) + 'static,
        {
            self.on_messages(move |cx, data| {
                if let Some(m) = cx.try_pop() {
                    f(cx.data(), data, m);
                }
            })
        }

        /// Assign a generic message handler
        ///
        /// `f` will be called whenever any message is available (see
        /// [`Widget::handle_messages`].
        pub fn on_messages<H>(mut self, f: H) -> Self
        where
            H: Fn(&mut EventCx<A>, &mut S) + 'static,
        {
            // While we could support multiple handlers there appears little reason.
            debug_assert!(self.on_messages.is_none(), "multiple message handlers assigned");

            self.on_messages = Some(Box::new(f));
            self
        }

        /// Assign an update handler
        ///
        /// This is called whenever the input data changes (and potentially
        /// more often); see [`Widget::update`].
        pub fn on_update<H>(mut self, f: H) -> Self
        where
            H: Fn(&mut ConfigCx<A>, &mut S) + 'static,
        {
            debug_assert!(self.on_update.is_none(), "multiple update handlers assigned");

            self.on_update = Some(Box::new(f));
            self
        }
    }

    impl Widget for Self {
        fn update(&mut self, cx: &mut ConfigCx<Self::Data>) {
            if let Some(ref h) = self.on_update {
                h(cx, &mut self.state);
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx<Self::Data>) {
            if let Some(ref mh) = self.on_messages {
                mh(cx, &mut self.state);
                cx.config_cx(|cx| cx.update(self));
            }
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
