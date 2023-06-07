// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event adapters

use kas::event::{ConfigCx, EventCx};
use kas::geom::{Offset, Size};
use kas::{autoimpl, impl_scope, Scrollable, Widget};

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
            self.inner.update(cx);
        }
    }

    // TODO: make derivable
    impl Scrollable for Self where W: Scrollable {
        #[inline]
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            self.inner.scroll_axes(size)
        }
        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.inner.max_scroll_offset()
        }
        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.inner.scroll_offset()
        }
        #[inline]
        fn set_scroll_offset(&mut self, cx: &mut EventCx<Self::Data>, offset: Offset) -> Offset {
            self.inner.set_scroll_offset(cx, offset)
        }
    }
}
