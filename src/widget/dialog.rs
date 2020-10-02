// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.

use kas::event::VirtualKeyCode;
use kas::prelude::*;
use kas::widget::{Label, TextButton};
use kas::WindowId;

#[derive(Clone, Debug, VoidMsg)]
enum DialogButton {
    Close,
}

/// A simple message box.
#[layout(column)]
#[widget(config=noauto)]
#[derive(Clone, Debug, Widget)]
pub struct MessageBox {
    #[widget_core]
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
    pub fn new<A: ToString, B: Into<FormattedString>>(title: A, message: B) -> Self {
        MessageBox {
            core: Default::default(),
            layout_data: Default::default(),
            title: title.to_string(),
            label: Label::from(message.into()),
            button: TextButton::new("Ok", DialogButton::Close).with_keys(&[
                VirtualKeyCode::Return,
                VirtualKeyCode::Space,
                VirtualKeyCode::NumpadEnter,
            ]),
        }
    }

    /// Construct from Markdown
    #[cfg(feature = "markdown")]
    pub fn from_md<A: ToString>(title: A, text: &str) -> Self {
        let text = kas::text::parser::Markdown::new(text);
        MessageBox::new(title, text)
    }
}

impl MessageBox {
    fn handle_button(&mut self, mgr: &mut Manager, msg: DialogButton) -> Response<VoidMsg> {
        match msg {
            DialogButton::Close => mgr.send_action(TkAction::Close),
        };
        Response::None
    }
}

impl kas::WidgetConfig for MessageBox {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.enable_alt_bypass(true);
    }
}

impl kas::Window for MessageBox {
    fn title(&self) -> &str {
        &self.title
    }

    fn restrict_dimensions(&self) -> (bool, bool) {
        (true, true)
    }

    // do not support overlays (yet?)
    fn add_popup(&mut self, _: &mut Manager, _: WindowId, _: kas::Popup) {
        panic!("MessageBox does not (currently) support pop-ups");
    }

    fn remove_popup(&mut self, _: &mut Manager, _: WindowId) {}
    fn resize_popups(&mut self, _: &mut dyn SizeHandle) {}
}
