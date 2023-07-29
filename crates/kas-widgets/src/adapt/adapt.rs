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
        message_handlers: Vec<Box<dyn Fn(&mut EventMgr, &A, &mut S) -> bool>>,
        update_handler: Option<Box<dyn Fn(&mut ConfigMgr, &A, &mut S)>>,
    }

    impl Self {
        /// Construct over `inner` with additional `state`
        #[inline]
        pub fn new(inner: W, state: S) -> Self {
            Adapt {
                core: Default::default(),
                state,
                inner,
                message_handlers: vec![],
                update_handler: None,
            }
        }

        /// Add a handler on message of type `M`
        ///
        /// Children will be updated whenever this handler is invoked.
        ///
        /// Where multiple message types must be handled or access to the
        /// [`EventMgr`] is required, use [`Self::on_messages`] instead.
        pub fn on_message<M, H>(self, handler: H) -> Self
        where
            M: Debug + 'static,
            H: Fn(&A, &mut S, M) + 'static,
        {
            self.on_messages(move |mgr, data, state| {
                if let Some(m) = mgr.try_pop() {
                    handler(data, state, m);
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
            H: Fn(&mut EventMgr, &A, &mut S) -> bool + 'static,
        {
            self.message_handlers.push(Box::new(handler));
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
            }
        }

        fn handle_messages(&mut self, data: &A, mgr: &mut EventMgr) {
            let mut update = false;
            for handler in self.message_handlers.iter() {
                update |= handler(mgr, data, &mut self.state);
            }
            if update {
                mgr.update(self.as_node_mut(data));
            }
        }
    }
}

impl_scope! {
    /// Data mapping
    ///
    /// This is a generic data-mapping widget. See also [`Adapt`], [`MapAny`].
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
