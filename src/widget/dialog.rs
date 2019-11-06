// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.

use crate::event::{Callback, Response};
use crate::geom::{AxisInfo, Coord, Rect, Size};
use crate::macros::Widget;
use crate::widget::{Label, TextButton};
use crate::{
    class::Class, CoreData, Layout, LayoutData, TkAction, TkWindow, Widget, WidgetId, Window,
};

/// A simple message box.
#[widget(class = Class::Window, layout = vertical)]
#[handler]
#[derive(Clone, Debug, Widget)]
pub struct MessageBox {
    #[core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as LayoutData>::Data,
    #[widget]
    label: Label,
    #[widget(handler = handle_button)]
    button: TextButton<()>,
}

impl MessageBox {
    pub fn new<T: ToString>(message: T) -> Self {
        MessageBox {
            core: Default::default(),
            layout_data: Default::default(),
            label: Label::new(message),
            button: TextButton::new("Ok", ()),
        }
    }

    fn handle_button(&mut self, tk: &mut dyn TkWindow, _msg: ()) -> Response<()> {
        tk.send_action(TkAction::Close);
        Response::None
    }
}

impl Window for MessageBox {
    fn as_widget(&self) -> &dyn Widget {
        self
    }
    fn as_widget_mut(&mut self) -> &mut dyn Widget {
        self
    }

    fn configure(&mut self) {
        let mut id = WidgetId::FIRST;
        self.walk(&mut |widget| {
            widget.core_data_mut().id = id;
            id = id.next();
        });
    }

    fn resize(&mut self, tk: &mut dyn TkWindow, size: Size) {
        // We call size_rules not because we want the result, but because our
        // spec requires that we do so before calling set_rect.
        let _ = self.size_rules(tk, AxisInfo::new(false, None));
        let _ = self.size_rules(tk, AxisInfo::new(true, Some(size.0)));
        let pos = Coord(0, 0);
        self.set_rect(Rect { pos, size });
    }

    // doesn't support callbacks, so doesn't need to do anything here
    fn callbacks(&self) -> Vec<(usize, Callback)> {
        Vec::new()
    }
    fn trigger_callback(&mut self, _index: usize, _tk: &mut dyn TkWindow) {}
}
