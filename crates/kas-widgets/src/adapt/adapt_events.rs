// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event adapters

use kas::event::ConfigCx;
use kas::{autoimpl, impl_scope, widget_index, Events, Widget};

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
        on_configure: Option<Box<dyn Fn(&mut ConfigCx, &mut W)>>,
        on_update: Option<Box<dyn Fn(&mut ConfigCx, &mut W, &W::Data)>>,
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
            }
        }

        /// Call the given closure on [`Events::configure`]
        ///
        /// Returns a wrapper around the input widget.
        #[must_use]
        pub fn on_configure<F>(mut self, f: F) -> Self
        where
            F: Fn(&mut ConfigCx, &mut W) + 'static,
        {
            self.on_configure = Some(Box::new(f));
            self
        }

        /// Call the given closure on [`Events::update`]
        ///
        /// Returns a wrapper around the input widget.
        #[must_use]
        pub fn on_update<F>(mut self, f: F) -> Self
        where
            F: Fn(&mut ConfigCx, &mut W, &W::Data) + 'static,
        {
            self.on_update = Some(Box::new(f));
            self
        }
    }

    impl Events for Self {
        type Data = W::Data;

        // This is a little bit hacky: the closures operate on self.inner, so we
        // need to ensure that is configured / updated first!

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            let id = self.make_child_id(widget_index!(self.inner));
            cx.configure(self.inner.as_node(data), id);
            if let Some(ref f) = self.on_configure {
                f(cx, &mut self.inner);
            }
            if let Some(ref f) = self.on_update {
                f(cx, &mut self.inner, data);
            }
        }

        fn update_recurse(&mut self, cx: &mut ConfigCx, data: &W::Data) {
            cx.update(self.inner.as_node(data));
            if let Some(ref f) = self.on_update {
                f(cx, &mut self.inner, data);
            }
        }
    }
}
