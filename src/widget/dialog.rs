// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.

use crate::event::{Callback, Response, VoidMsg};
use crate::geom::Size;
use crate::layout;
use crate::macros::{VoidMsg, Widget};
use crate::theme::SizeHandle;
use crate::widget::{Label, TextButton};
use crate::{CoreData, TkAction, TkWindow, Window};

#[derive(Clone, Debug, VoidMsg)]
enum DialogButton {
    Close,
}

/// A simple message box.
#[widget(layout = vertical)]
#[handler]
#[derive(Clone, Debug, Widget)]
pub struct MessageBox {
    #[core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    title: String,
    #[widget]
    label: Label,
    #[widget(handler = handle_button)]
    button: TextButton<DialogButton>,
}

impl MessageBox {
    pub fn new<T: ToString, M: ToString>(title: T, message: M) -> Self {
        MessageBox {
            core: Default::default(),
            layout_data: Default::default(),
            title: title.to_string(),
            label: Label::new(message),
            button: TextButton::new("Ok", DialogButton::Close),
        }
    }

    fn handle_button(&mut self, tk: &mut dyn TkWindow, msg: DialogButton) -> Response<VoidMsg> {
        match msg {
            DialogButton::Close => tk.send_action(TkAction::Close),
        };
        Response::None
    }
}

impl Window for MessageBox {
    fn title(&self) -> &str {
        &self.title
    }

    fn resize(&mut self, size_handle: &mut dyn SizeHandle, size: Size) {
        layout::solve(self, size_handle, size);
    }

    // doesn't support callbacks, so doesn't need to do anything here
    fn callbacks(&self) -> Vec<(usize, Callback)> {
        Vec::new()
    }
    fn trigger_callback(&mut self, _index: usize, _tk: &mut dyn TkWindow) {}
}
