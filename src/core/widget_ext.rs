// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget extension traits

use super::{MsgMapWidget, Widget};
use crate::event::{Manager, Response};

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
