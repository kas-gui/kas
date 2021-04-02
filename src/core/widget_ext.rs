// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget extension traits

use super::Widget;
use crate::adapter::{MapResponse, Reserve, WithLabel};
use crate::dir::Directional;
use crate::draw::SizeHandle;
use crate::event::{Manager, Response};
use crate::layout::{AxisInfo, SizeRules};
use crate::text::AccelString;
#[allow(unused)]
use kas::Layout;

/// Provides some convenience methods on widgets
pub trait WidgetExt: Widget {
    /// Construct a wrapper widget which maps messages from this widget
    ///
    /// Responses from this widget with a message payload are mapped with `f`.
    fn map_msg<F, M>(self, f: F) -> MapResponse<Self, M>
    where
        F: Fn(&mut Manager, Self::Msg) -> M + 'static,
        Self: Sized,
    {
        MapResponse::new(self, move |mgr, msg| Response::Msg(f(mgr, msg)))
    }

    /// Construct a wrapper widget which discards messages from this widget
    ///
    /// Responses from this widget with a message payload are mapped to
    /// [`Response::None`].
    fn map_msg_discard<M>(self) -> MapResponse<Self, M>
    where
        Self: Sized,
    {
        MapResponse::new(self, |_, _| Response::None)
    }

    /// Construct a wrapper widget which maps message responses from this widget
    ///
    /// Responses from this widget with a message payload are mapped with `f`.
    fn map_response<F, M>(self, f: F) -> MapResponse<Self, M>
    where
        F: Fn(&mut Manager, Self::Msg) -> Response<M> + 'static,
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
    /// use kas::widget::Label;
    /// use kas::prelude::*;
    ///
    /// let label = Label::new("0").with_reserve(|size_handle, axis| {
    ///     Label::new("00000").size_rules(size_handle, axis)
    /// });
    ///```
    /// Alternatively one may use virtual pixels:
    ///```
    /// use kas::widget::Filler;
    /// use kas::prelude::*;
    ///
    /// let label = Filler::new().with_reserve(|size_handle, axis| {
    ///     let size = size_handle.scale_factor() * 100.0;
    ///     SizeRules::fixed(size.cast_nearest(), (0, 0))
    /// });
    ///```
    /// The resulting `SizeRules` will be the max of those for the inner widget
    /// and the result of the `reserve` closure.
    fn with_reserve<R>(self, r: R) -> Reserve<Self, R>
    where
        R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static,
        Self: Sized,
    {
        Reserve::new(self, r)
    }

    /// Construct a wrapper widget adding a label
    fn with_label<D, T>(self, direction: D, label: T) -> WithLabel<Self, D>
    where
        D: Directional,
        T: Into<AccelString>,
        Self: Sized,
    {
        WithLabel::new_with_direction(direction, self, label)
    }
}
impl<W: Widget + ?Sized> WidgetExt for W {}

/// Provides a convenient `.boxed()` method on implementors
//
// Note: this is distinct from WidgetExt to allow this variant on M: Menu + Sized:
// fn boxed(self) -> Box<dyn Menu<Msg = M::Msg>>
pub trait Boxed<T: ?Sized> {
    /// Boxing method
    fn boxed(self) -> Box<T>;
}

impl<W: Widget + Sized> Boxed<dyn Widget<Msg = W::Msg>> for W {
    fn boxed(self) -> Box<dyn Widget<Msg = W::Msg>> {
        Box::new(self)
    }
}
