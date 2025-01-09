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
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.inner.set_rect(cx, rect, self.hints.combine(hints));
        }
    }
}

impl_scope! {
    /// Apply an alignment hint, squash and align the result
    ///
    /// The inner widget chooses how to apply (or ignore) this hint.
    /// The widget is then prevented from stretching beyond its ideal size,
    /// aligning within the available rect.
    ///
    /// Usually, this type will be constructed through one of the methods on
    /// [`AdaptWidget`](crate::adapt::AdaptWidget).
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[widget{ derive = self.inner; }]
    pub struct Pack<W: Widget> {
        pub inner: W,
        /// Hints may be modified directly.
        ///
        /// Use [`Action::RESIZE`] to apply changes.
        pub hints: AlignHints,
        size: Size,
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new(inner: W, hints: AlignHints) -> Self {
            Pack { inner, hints, size: Size::ZERO }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let rules = self.inner.size_rules(sizer, axis);
            self.size.set_component(axis, rules.ideal_size());
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            let align = self.hints.combine(hints).complete_default();
            let rect = align.aligned_rect(self.size, rect);
            self.inner.set_rect(cx, rect, hints);
        }
    }
}
