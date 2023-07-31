// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event adapters

use kas::event::ConfigCx;
use kas::{autoimpl, impl_scope, Events, Widget};

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
        f: Box<dyn Fn(&mut ConfigCx, &mut W, &W::Data)>,
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new<F: Fn(&mut ConfigCx, &mut W, &W::Data) + 'static>(inner: W, f: F) -> Self {
            OnUpdate { core: Default::default(), inner, f: Box::new(f) }
        }
    }

    impl Events for Self {
        type Data = W::Data;

        fn update(&mut self, cx: &mut ConfigCx, data: &W::Data) {
            (self.f)(cx, &mut self.inner, data);
        }
    }
}
