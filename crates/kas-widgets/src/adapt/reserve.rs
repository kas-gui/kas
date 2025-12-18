// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Size reservation

#[allow(unused)] use crate::adapt::AdaptWidget;
use kas::prelude::*;
use kas::theme::MarginStyle;

#[impl_self]
mod Reserve {
    /// A generic widget for size reservations
    ///
    /// In a few cases it is desirable to reserve more space for a widget than
    /// required for the current content, e.g. if a label's text may change. This
    /// widget can be used for this by wrapping the base widget.
    ///
    /// Usually, this type will be constructed through one of the methods on
    /// [`AdaptWidget`].
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(Viewport using self.inner where W: trait)]
    #[derive_widget]
    pub struct Reserve<W: Widget> {
        #[widget]
        pub inner: W,
        reserve: Box<dyn Fn(&mut SizeCx, AxisInfo) -> SizeRules + 'static>,
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
        /// let label = Reserve::new(Filler::new(), |cx: &mut SizeCx<'_>, axis| {
        ///     cx.logical(100.0, 100.0).build(axis)
        /// });
        ///```
        /// The resulting `SizeRules` will be the max of those for the inner widget
        /// and the result of the `reserve` closure.
        #[inline]
        pub fn new(
            inner: W,
            reserve: impl Fn(&mut SizeCx, AxisInfo) -> SizeRules + 'static,
        ) -> Self {
            let reserve = Box::new(reserve);
            Reserve { inner, reserve }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let inner_rules = self.inner.size_rules(cx, axis);
            let reserve_rules = (self.reserve)(cx, axis);
            inner_rules.max(reserve_rules)
        }
    }
}

#[impl_self]
mod WithMarginStyle {
    /// Specify margins via a style
    ///
    /// This replaces a widget's margins.
    ///
    /// Usually, this type will be constructed using
    /// [`AdaptWidget::with_margin_style`].
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(Viewport using self.inner where W: trait)]
    #[derive_widget]
    pub struct WithMarginStyle<W: Widget> {
        #[widget]
        pub inner: W,
        style: MarginStyle,
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new(inner: W, style: MarginStyle) -> Self {
            WithMarginStyle { inner, style }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let child_rules = self.inner.size_rules(cx, axis);
            let margins = cx.margins(self.style).extract(axis);
            child_rules.with_margins(margins)
        }
    }
}
