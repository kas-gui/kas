// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event adapters

use kas::event::{ConfigMgr, EventMgr};
use kas::geom::{Offset, Size};
use kas::{autoimpl, impl_scope, Events, Scrollable, Widget};

impl_scope! {
    /// Wrapper to call a closure on update
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[widget {
        layout = self.inner;
    }]
    pub struct OnUpdate<W: Widget> {
        core: widget_core!(),
        #[widget]
        pub inner: W,
        f: Box<dyn Fn(&mut ConfigMgr, &mut W, &W::Data)>,
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new<F: Fn(&mut ConfigMgr, &mut W, &W::Data) + 'static>(inner: W, f: F) -> Self {
            OnUpdate { core: Default::default(), inner, f: Box::new(f) }
        }
    }

    impl Events for Self {
        type Data = W::Data;

        fn update(&mut self, data: &W::Data, cx: &mut ConfigMgr) {
            (self.f)(cx, &mut self.inner, data);
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
        fn set_scroll_offset(&mut self, cx: &mut EventMgr, offset: Offset) -> Offset {
            self.inner.set_scroll_offset(cx, offset)
        }
    }
}
