// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Alignment

use kas::layout::AlignHints;
use kas::prelude::*;

impl_scope! {
    /// Wrapper to apply alignment
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Debug, Default)]
    #[widget{ derive = self.inner; }]
    pub struct WithAlign<W: Widget> {
        pub inner: W,
        hints: AlignHints,
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new(inner: W, horiz: Option<Align>, vert: Option<Align>) -> Self {
            let hints = AlignHints { horiz, vert };
            WithAlign { inner, hints }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let axis = axis.with_align_hints(self.hints);
            self.inner.size_rules(size_mgr, axis)
        }
    }
}
