// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Alignment

use kas::prelude::*;

impl_scope! {
    /// Apply an alignment hint
    ///
    /// The inner widget chooses how to apply (or ignore) this hint.
    ///
    /// Usually, this type will be constructed through one of the methods on
    /// [`AdaptWidget`](crate::adapt::AdaptWidget).
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[widget{ derive = self.inner; }]
    pub struct Align<W: Widget> {
        pub inner: W,
        /// Hints may be modified directly.
        ///
        /// Use [`Action::RESIZE`] to apply changes.
        pub hints: AlignHints,
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new(inner: W, hints: AlignHints) -> Self {
            Align { inner, hints }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.inner
            .size_rules(sizer, axis.with_align_hints(self.hints))
        }
    }
}
