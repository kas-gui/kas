// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Size reservation

use kas::prelude::*;

impl_scope! {
    /// A generic widget for size reservations
    ///
    /// In a few cases it is desirable to reserve more space for a widget than
    /// required for the current content, e.g. if a label's text may change. This
    /// widget can be used for this by wrapping the base widget.
    ///
    /// Usually, this type will be constructed through one of the methods on
    /// [`AdaptWidget`](crate::adapt::AdaptWidget).
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[widget{ derive = self.inner; }]
    pub struct Reserve<W: Widget> {
        pub inner: W,
        reserve: Box<dyn Fn(SizeCx, AxisInfo) -> SizeRules + 'static>,
    }

    impl Self {
        /// Construct a reserve
        ///
        /// The closure `reserve` should generate `SizeRules` on request, just like
        /// [`Layout::size_rules`]. For example:
        ///```
        /// use kas_widgets::adapt::Reserve;
        /// use kas_widgets::Filler;
        /// use kas::prelude::*;
        ///
        /// let label = Reserve::new(Filler::new(), |sizer: SizeCx<'_>, axis| {
        ///     let size = i32::conv_ceil(sizer.scale_factor() * 100.0);
        ///     SizeRules::fixed(size, (0, 0))
        /// });
        ///```
        /// The resulting `SizeRules` will be the max of those for the inner widget
        /// and the result of the `reserve` closure.
        #[inline]
        pub fn new(inner: W, reserve: impl Fn(SizeCx, AxisInfo) -> SizeRules + 'static) -> Self {
            let reserve = Box::new(reserve);
            Reserve { inner, reserve }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let inner_rules = self.inner.size_rules(sizer.re(), axis);
            let reserve_rules = (self.reserve)(sizer.re(), axis);
            inner_rules.max(reserve_rules)
        }
    }
}
