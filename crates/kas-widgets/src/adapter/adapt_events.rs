// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event adapters

use kas::event::ConfigCx;
use kas::Widget;
use kas::{autoimpl, impl_scope};

impl_scope! {
    /// Wrapper to call a closure on update
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(Debug ignore self.f)]
    #[widget{ derive = self.inner; }]
    pub struct OnUpdate<W: Widget, F: Fn(&mut W, &mut ConfigCx<W::Data>)> {
        pub inner: W,
        f: F,
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new(inner: W, f: F) -> Self {
            OnUpdate { inner, f }
        }
    }

    impl Widget for Self {
        fn update(&mut self, cx: &mut ConfigCx<W::Data>) {
            (self.f)(&mut self.inner, cx);
        }
    }
}
