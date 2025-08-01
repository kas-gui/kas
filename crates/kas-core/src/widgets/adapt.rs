// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapter widgets (wrappers)

#[allow(unused)] use crate::Action;
use crate::event::ConfigCx;
use crate::geom::{Rect, Size};
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::theme::SizeCx;
use crate::{Layout, Widget};
use kas_macros::{autoimpl, impl_self};

#[impl_self]
mod MapAny {
    /// Map any input data to `()`
    ///
    /// This is a generic data-mapping widget-wrapper with fixed `()` input
    /// data type.
    ///
    /// This struct is a thin wrapper around the inner widget without its own
    /// [`Id`](crate::Id). It supports [`Deref`](std::ops::Deref) and
    /// [`DerefMut`](std::ops::DerefMut) to the inner widget.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(Clone, Default where W: trait)]
    #[autoimpl(Scrollable using self.inner where W: trait)]
    #[derive_widget(type Data = A)]
    pub struct MapAny<A, W: Widget<Data = ()>> {
        _a: std::marker::PhantomData<A>,
        /// The inner widget
        #[widget(&())]
        pub inner: W,
    }

    impl Self {
        /// Construct
        pub fn new(inner: W) -> Self {
            MapAny {
                _a: std::marker::PhantomData,
                inner,
            }
        }
    }
}

#[impl_self]
mod Align {
    /// Apply an alignment hint
    ///
    /// The inner widget chooses how to apply (or ignore) this hint.
    ///
    /// Usually, this type will be constructed through one of the methods on
    /// [`AdaptWidget`](https://docs.rs/kas/latest/kas/widgets/trait.AdaptWidget.html).
    #[derive_widget]
    pub struct Align<W: Widget> {
        #[widget]
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

#[impl_self]
mod Pack {
    /// Apply an alignment hint, squash and align the result
    ///
    /// The inner widget chooses how to apply (or ignore) this hint.
    /// The widget is then prevented from stretching beyond its ideal size,
    /// aligning within the available rect.
    ///
    /// Usually, this type will be constructed through one of the methods on
    /// [`AdaptWidget`](https://docs.rs/kas/latest/kas/widgets/trait.AdaptWidget.html).
    #[derive_widget]
    pub struct Pack<W: Widget> {
        #[widget]
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
            Pack {
                inner,
                hints,
                size: Size::ZERO,
            }
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
