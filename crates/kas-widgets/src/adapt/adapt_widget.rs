// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget extension traits

use super::{FnSizeRules, Map, MapAny, OnUpdate, Reserve, WithLabel};
use kas::cast::{Cast, CastFloat};
use kas::dir::Directional;
use kas::event::ConfigCx;
use kas::geom::Vec2;
use kas::layout::{AxisInfo, SizeRules};
use kas::text::AccelString;
use kas::theme::SizeCx;
use kas::Widget;
#[allow(unused)] use kas::{Events, Layout};

/// Support type for [`AdaptWidget::with_min_size_px`]
pub struct WithMinSizePx(Vec2);
impl FnSizeRules for WithMinSizePx {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let size = self.0.extract(axis) * sizer.scale_factor();
        SizeRules::fixed(size.cast_ceil(), (0, 0))
    }
}

/// Support type for [`AdaptWidget::with_min_size_em`]
pub struct WithMinSizeEm(Vec2);
impl FnSizeRules for WithMinSizeEm {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let size = self.0.extract(axis) * sizer.dpem();
        SizeRules::fixed(size.cast_ceil(), (0, 0))
    }
}

/// Provides `.map_any()`
///
/// TODO: move to `AdaptWidget` with `where Self::Data == ()` constraint
/// once supported (Rust#20041).
pub trait AdaptWidgetAny: Widget<Data = ()> + Sized {
    /// Map any input data to `()`
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn map_any<A>(self) -> MapAny<A, Self> {
        MapAny::new(self)
    }
}
impl<W: Widget<Data = ()>> AdaptWidgetAny for W {}

/// Provides some convenience methods on widgets
pub trait AdaptWidget: Widget + Sized {
    /// Map data type via a function
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn map<A, F>(self, f: F) -> Map<A, Self, F>
    where
        F: for<'a> Fn(&'a A) -> &'a Self::Data,
    {
        Map::new(self, f)
    }

    /// Call the given closure on [`Events::configure`]
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn on_configure<F>(self, f: F) -> OnUpdate<Self>
    where
        F: Fn(&mut ConfigCx, &mut Self) + 'static,
    {
        OnUpdate::new(self).on_configure(f)
    }

    /// Call the given closure on [`Events::update`]
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn on_update<F>(self, f: F) -> OnUpdate<Self>
    where
        F: Fn(&mut ConfigCx, &mut Self, &Self::Data) + 'static,
    {
        OnUpdate::new(self).on_update(f)
    }

    /// Construct a wrapper widget which reserves extra space
    ///
    /// The closure `reserve` should generate `SizeRules` on request, just like
    /// [`Layout::size_rules`]. This can be done by instantiating a temporary
    /// widget, for example:
    ///```
    /// # use kas_widgets::adapt::AdaptWidget;
    /// use kas::prelude::*;
    /// use kas_widgets::Label;
    ///
    /// let label = Label::new("0")
    ///     .with_reserve(|sizer, axis| Label::new("00000").size_rules(sizer, axis));
    /// ```
    /// Alternatively one may use virtual pixels:
    ///```
    /// # use kas_widgets::adapt::AdaptWidget;
    /// use kas::prelude::*;
    /// use kas_widgets::Filler;
    ///
    /// let label = Filler::new().with_reserve(|cx, axis| {
    ///     kas::layout::LogicalSize(5.0, 5.0).to_rules(axis, cx.scale_factor())
    /// });
    /// ```
    /// The resulting `SizeRules` will be the max of those for the inner widget
    /// and the result of the `reserve` closure.
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn with_reserve<R>(self, r: R) -> Reserve<Self, R>
    where
        R: FnMut(SizeCx, AxisInfo) -> SizeRules,
    {
        Reserve::new(self, r)
    }

    /// Construct a wrapper, setting minimum size in pixels
    ///
    /// The input size is scaled by the scale factor.
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn with_min_size_px(self, w: i32, h: i32) -> Reserve<Self, WithMinSizePx> {
        let size = Vec2(w.cast(), h.cast());
        Reserve::new(self, WithMinSizePx(size))
    }

    /// Construct a wrapper, setting minimum size in Em
    ///
    /// This depends on the font size, though not the exact font in use.
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn with_min_size_em(self, w: f32, h: f32) -> Reserve<Self, WithMinSizeEm> {
        let size = Vec2(w, h);
        Reserve::new(self, WithMinSizeEm(size))
    }

    /// Construct a wrapper widget adding a label
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn with_label<D, T>(self, direction: D, label: T) -> WithLabel<Self, D>
    where
        D: Directional,
        T: Into<AccelString>,
    {
        WithLabel::new_dir(self, direction, label)
    }
}
impl<W: Widget> AdaptWidget for W {}
