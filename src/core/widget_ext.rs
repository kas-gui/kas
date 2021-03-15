// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget extension traits

use super::{MsgMapWidget, Widget};
use crate::draw::SizeHandle;
use crate::event::{Manager, Response};
use crate::layout::{AxisInfo, SizeRules};
use kas::widget::Reserve;

/// Provides some convenience methods on widgets
pub trait WidgetExt: Widget {
    /// Construct a wrapper widget which maps messages from this widget
    ///
    /// Responses from this widget with a message payload are mapped with `f`.
    fn map_msg<F: Fn(&mut Manager, Self::Msg) -> M + 'static, M>(
        self,
        f: F,
    ) -> MsgMapWidget<Self, M>
    where
        Self: Sized,
    {
        MsgMapWidget::new(self, move |mgr, msg| Response::Msg(f(mgr, msg)))
    }

    /// Construct a wrapper widget which discards messages from this widget
    ///
    /// Responses from this widget with a message payload are mapped to
    /// [`Response::None`].
    fn map_msg_discard<M>(self) -> MsgMapWidget<Self, M>
    where
        Self: Sized,
    {
        MsgMapWidget::new(self, |_, _| Response::None)
    }

    /// Construct a wrapper widget which maps message responses from this widget
    ///
    /// Responses from this widget with a message payload are mapped with `f`.
    fn map_response<F: Fn(&mut Manager, Self::Msg) -> Response<M> + 'static, M>(
        self,
        f: F,
    ) -> MsgMapWidget<Self, M>
    where
        Self: Sized,
    {
        MsgMapWidget::new(self, f)
    }

    /// Construct a wrapper widget which reserves extra space
    ///
    /// The closure `reserve` should generate `SizeRules` on request, just like
    /// [`Layout::size_rules`]. This can be done by instantiating a temporary
    /// widget, for example:
    ///```
    /// use kas::widget::{Reserve, Label};
    /// use kas::prelude::*;
    ///
    /// let label = Reserve::new(Label::new("0"), |size_handle, axis| {
    ///     Label::new("00000").size_rules(size_handle, axis)
    /// });
    ///```
    /// Alternatively one may use virtual pixels:
    ///```
    /// use kas::widget::{Reserve, Filler};
    /// use kas::prelude::*;
    ///
    /// let label = Reserve::new(Filler::new(), |size_handle, axis| {
    ///     let size = i32::conv_ceil(size_handle.scale_factor() * 100.0);
    ///     SizeRules::fixed(size, (0, 0))
    /// });
    ///```
    /// The resulting `SizeRules` will be the max of those for the inner widget
    /// and the result of the `reserve` closure.
    fn reserve<R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static>(
        self,
        r: R,
    ) -> Reserve<Self, R>
    where
        Self: Sized,
    {
        Reserve::new(self, r)
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
