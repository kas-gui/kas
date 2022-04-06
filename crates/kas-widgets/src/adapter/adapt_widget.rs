// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget extension traits

use super::{MapResponse, Reserve, WithLabel};
use kas::dir::Directional;
use kas::event::{EventMgr, Response, VoidMsg};
use kas::layout::{AxisInfo, SizeRules};
use kas::text::AccelString;
use kas::theme::SizeMgr;
#[allow(unused)]
use kas::Layout;
use kas::Widget;

/// Provides some convenience methods on widgets
pub trait AdaptWidget: Widget {
    /// Construct a wrapper widget which maps messages to `M`
    ///
    /// Responses from this widget with a message payload are mapped with `f`.
    #[must_use]
    fn map_msg<F, M>(self, f: F) -> MapResponse<Self, M>
    where
        F: Fn(&mut EventMgr, Self::Msg) -> M + 'static,
        Self: Sized,
    {
        MapResponse::new(self, move |mgr, msg| Response::Msg(f(mgr, msg)))
    }

    /// Construct a wrapper widget which maps messages from [`VoidMsg`] to `M`
    ///
    /// Responses from this widget with a message payload are mapped with `f`.
    #[must_use]
    fn map_void_msg<M>(self) -> MapResponse<Self, M>
    where
        Self: Widget<Msg = VoidMsg> + Sized,
    {
        MapResponse::new(self, |_, msg| match msg {})
    }

    /// Construct a wrapper widget which discards messages from this widget
    ///
    /// Responses from this widget with a message payload are mapped to
    /// [`Response::Used`].
    #[must_use]
    fn map_msg_discard<M>(self) -> MapResponse<Self, M>
    where
        Self: Sized,
    {
        MapResponse::new(self, |_, _| Response::Used)
    }

    /// Construct a wrapper widget which maps message responses from this widget
    ///
    /// Responses from this widget with a message payload are mapped with `f`.
    #[must_use]
    fn map_response<F, M>(self, f: F) -> MapResponse<Self, M>
    where
        F: Fn(&mut EventMgr, Self::Msg) -> Response<M> + 'static,
        Self: Sized,
    {
        MapResponse::new(self, f)
    }

    /// Construct a wrapper widget which reserves extra space
    ///
    /// The closure `reserve` should generate `SizeRules` on request, just like
    /// [`Layout::size_rules`]. This can be done by instantiating a temporary
    /// widget, for example:
    ///```
    /// # use kas_widgets::adapter::AdaptWidget;
    /// use kas_widgets::Label;
    /// use kas::prelude::*;
    ///
    /// let label = Label::new("0").with_reserve(|size_mgr, axis| {
    ///     Label::new("00000").size_rules(size_mgr, axis)
    /// });
    ///```
    /// Alternatively one may use virtual pixels:
    ///```
    /// # use kas_widgets::adapter::AdaptWidget;
    /// use kas_widgets::Filler;
    /// use kas::prelude::*;
    ///
    /// let label = Filler::new().with_reserve(|mgr, axis| {
    ///     LogicalSize(5.0, 5.0).to_rules(axis, mgr.scale_factor())
    /// });
    ///```
    /// The resulting `SizeRules` will be the max of those for the inner widget
    /// and the result of the `reserve` closure.
    #[must_use]
    fn with_reserve<R>(self, r: R) -> Reserve<Self, R>
    where
        R: FnMut(SizeMgr, AxisInfo) -> SizeRules + 'static,
        Self: Sized,
    {
        Reserve::new(self, r)
    }

    /// Construct a wrapper widget adding a label
    #[must_use]
    fn with_label<D, T>(self, direction: D, label: T) -> WithLabel<Self, D>
    where
        D: Directional,
        T: Into<AccelString>,
        Self: Sized,
    {
        WithLabel::new_with_direction(direction, self, label)
    }
}
impl<W: Widget + ?Sized> AdaptWidget for W {}
