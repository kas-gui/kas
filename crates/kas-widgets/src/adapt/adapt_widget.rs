// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget extension traits

use super::{AdaptConfigCx, AdaptEventCx, AdaptEvents, Map, MapAny, Reserve, WithLabel};
use kas::cast::{Cast, CastFloat};
use kas::dir::Directional;
use kas::geom::Vec2;
use kas::layout::{AxisInfo, SizeRules};
use kas::text::AccessString;
use kas::theme::SizeCx;
#[allow(unused)] use kas::Events;
use kas::Widget;
use std::fmt::Debug;

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
    fn on_configure<F>(self, f: F) -> AdaptEvents<Self>
    where
        F: Fn(&mut AdaptConfigCx, &mut Self) + 'static,
    {
        AdaptEvents::new(self).on_configure(f)
    }

    /// Call the given closure on [`Events::update`]
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn on_update<F>(self, f: F) -> AdaptEvents<Self>
    where
        F: Fn(&mut AdaptConfigCx, &mut Self, &Self::Data) + 'static,
    {
        AdaptEvents::new(self).on_update(f)
    }

    /// Add a handler on message of type `M`
    ///
    /// Where access to input data is required, use [`Self::on_messages`] instead.
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn on_message<M, H>(self, handler: H) -> AdaptEvents<Self>
    where
        M: Debug + 'static,
        H: Fn(&mut AdaptEventCx, &mut Self, M) + 'static,
    {
        AdaptEvents::new(self).on_message(handler)
    }

    /// Add a child handler to map messages of type `M` to `N`
    ///
    /// # Example
    ///
    /// ```
    /// use kas::messages::Select;
    /// use kas_widgets::{AdaptWidget, Row, Tab};
    ///
    /// #[derive(Clone, Debug)]
    /// struct MsgSelectIndex(usize);
    ///
    /// let tabs = Row::new([Tab::new("A")])
    ///     .map_message(|index, Select| MsgSelectIndex(index));
    /// ```
    fn map_message<M, N, H>(self, handler: H) -> AdaptEvents<Self>
    where
        M: Debug + 'static,
        N: Debug + 'static,
        H: Fn(usize, M) -> N + 'static,
    {
        AdaptEvents::new(self).map_message(handler)
    }

    /// Add a generic message handler
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn on_messages<H>(self, handler: H) -> AdaptEvents<Self>
    where
        H: Fn(&mut AdaptEventCx, &mut Self, &Self::Data) + 'static,
    {
        AdaptEvents::new(self).on_messages(handler)
    }

    /// Construct a wrapper, setting minimum size in pixels
    ///
    /// The input size is scaled by the scale factor.
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn with_min_size_px(self, w: i32, h: i32) -> Reserve<Self> {
        let size = Vec2(w.cast(), h.cast());
        Reserve::new(self, move |sizer: SizeCx, axis: AxisInfo| {
            let size = size.extract(axis) * sizer.scale_factor();
            SizeRules::fixed(size.cast_ceil(), (0, 0))
        })
    }

    /// Construct a wrapper, setting minimum size in Em
    ///
    /// This depends on the font size, though not the exact font in use.
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn with_min_size_em(self, w: f32, h: f32) -> Reserve<Self> {
        let size = Vec2(w, h);
        Reserve::new(self, move |sizer: SizeCx, axis: AxisInfo| {
            let size = size.extract(axis) * sizer.dpem();
            SizeRules::fixed(size.cast_ceil(), (0, 0))
        })
    }

    /// Construct a wrapper widget adding a label
    ///
    /// Returns a wrapper around the input widget.
    #[must_use]
    fn with_label<D, T>(self, direction: D, label: T) -> WithLabel<Self, D>
    where
        D: Directional,
        T: Into<AccessString>,
    {
        WithLabel::new_dir(self, direction, label)
    }
}
impl<W: Widget> AdaptWidget for W {}
