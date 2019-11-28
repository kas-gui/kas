// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.

use crate::event::{Callback, EmptyMsg};
use crate::geom::{Coord, Rect, Size};
use crate::layout;
use crate::macros::{EmptyMsg, Widget};
use crate::widget::{Label, TextButton};
use crate::{class::Class, CoreData, Layout, LayoutData, TkAction, TkWindow, Window};

#[derive(Clone, Debug, EmptyMsg)]
enum DialogButton {
    None,
    Close,
}

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
    button: TextButton<DialogButton>,
}

impl MessageBox {
    pub fn new<T: ToString>(message: T) -> Self {
        MessageBox {
            core: Default::default(),
            layout_data: Default::default(),
            label: Label::new(message),
            button: TextButton::new("Ok", DialogButton::Close),
        }
    }

    fn handle_button(&mut self, tk: &mut dyn TkWindow, msg: DialogButton) -> EmptyMsg {
        match msg {
            DialogButton::None => (),
            DialogButton::Close => tk.send_action(TkAction::Close),
        };
        EmptyMsg
    }
}

impl Window for MessageBox {
    fn resize(&mut self, tk: &mut dyn TkWindow, size: Size) {
        layout::solve(self, tk, size);
        let pos = Coord(0, 0);
        self.set_rect(tk, Rect { pos, size });
    }

    // doesn't support callbacks, so doesn't need to do anything here
    fn callbacks(&self) -> Vec<(usize, Callback)> {
        Vec::new()
    }
    fn trigger_callback(&mut self, _index: usize, _tk: &mut dyn TkWindow) {}
}
