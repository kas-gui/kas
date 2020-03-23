// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.

use kas::draw::SizeHandle;
use kas::event::{Callback, Manager, Response, VoidMsg};
use kas::layout;
use kas::prelude::*;
use kas::widget::{Label, TextButton};

#[derive(Clone, Debug, VoidMsg)]
enum DialogButton {
    Close,
}

/// A simple message box.
#[widget_config]
#[layout(vertical)]
#[handler]
#[derive(Clone, Debug, Widget)]
pub struct MessageBox {
    #[widget_core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    title: CowString,
    #[widget]
    label: Label,
    #[widget(handler = handle_button)]
    button: TextButton<DialogButton>,
}

impl MessageBox {
    pub fn new<T: Into<CowString>, M: Into<CowString>>(title: T, message: M) -> Self {
        MessageBox {
            core: Default::default(),
            layout_data: Default::default(),
            title: title.into(),
            label: Label::new(message),
            button: TextButton::new("Ok", DialogButton::Close),
        }
    }

    fn handle_button(&mut self, mgr: &mut Manager, msg: DialogButton) -> Response<VoidMsg> {
        match msg {
            DialogButton::Close => mgr.send_action(TkAction::Close),
        };
        Response::None
    }
}

impl kas::Window for MessageBox {
    fn title(&self) -> &str {
        &self.title
    }

    fn find_size(&mut self, size_handle: &mut dyn SizeHandle) -> (Option<Size>, Size) {
        let (min, ideal) = layout::solve(self, size_handle);
        (Some(min), ideal)
    }

    fn resize(
        &mut self,
        size_handle: &mut dyn SizeHandle,
        size: Size,
    ) -> (Option<Size>, Option<Size>) {
        let (min, ideal) = layout::solve_and_set(self, size_handle, size);
        (Some(min), Some(ideal))
    }

    // doesn't support callbacks, so doesn't need to do anything here
    fn callbacks(&self) -> Vec<(usize, Callback)> {
        Vec::new()
    }
    fn trigger_callback(&mut self, _index: usize, _: &mut Manager) {}
}
