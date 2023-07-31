// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Size reservation

use kas::prelude::*;

/// Requirements on parameter to [`Reserve`]
///
/// Note: this type is implemented for the expected [`FnMut`],
/// i.e. methods and closures are supported.
/// A trait is used to support custom *named* objects since currently closures
/// can't be named and trait methods cannot return unnamed objects (impl Trait
/// is only supported on functions and inherent methods).
pub trait FnSizeRules {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules;
}

impl<F> FnSizeRules for F
where
    for<'a> F: FnMut(SizeCx<'a>, AxisInfo) -> SizeRules,
{
    #[inline]
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self(sizer, axis)
    }
}

/// Parameterisation of [`Reserve`] using a function pointer
///
/// Since it is impossible to name closures, using [`Reserve`] where a type is
/// required (e.g. in a struct field) is only possible by making the containing
/// struct generic over this field, which may be undesirable. As an alternative
/// a function pointer may be preferred.
pub type ReserveP<W> = Reserve<W, fn(SizeCx, AxisInfo) -> SizeRules>;

impl_scope! {
    /// A generic widget for size reservations
    ///
    /// In a few cases it is desirable to reserve more space for a widget than
    /// required for the current content, e.g. if a label's text may change. This
    /// widget can be used for this by wrapping the base widget.
    ///
    /// Usually, this type will be constructed through one of the methods on
    /// [`AdaptWidget`](crate::adapter::AdaptWidget).
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Default)]
    #[widget{ derive = self.inner; }]
    pub struct Reserve<W: Widget, R: FnSizeRules> {
        pub inner: W,
        reserve: R,
    }

    impl Self {
        /// Construct a reserve
        ///
        /// The closure `reserve` should generate `SizeRules` on request, just like
        /// [`Layout::size_rules`]. This can be done by instantiating a temporary
        /// widget, for example:
        ///```
        /// use kas_widgets::adapt::Reserve;
        /// use kas_widgets::Label;
        /// use kas::prelude::*;
        ///
        /// let label = Reserve::new(Label::new("0"), |sizer: SizeCx<'_>, axis| {
        ///     Label::new("00000").size_rules(sizer, axis)
        /// });
        ///```
        /// Alternatively one may use virtual pixels:
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
        pub fn new(inner: W, reserve: R) -> Self {
            Reserve { inner, reserve }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let inner_rules = self.inner.size_rules(sizer.re(), axis);
            let reserve_rules = self.reserve.size_rules(sizer.re(), axis);
            inner_rules.max(reserve_rules)
        }
    }
}
