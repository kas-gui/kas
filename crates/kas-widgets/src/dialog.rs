// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.

use crate::{Label, TextButton};
use kas::event::VirtualKeyCode;
use kas::prelude::*;
use kas::text::format::FormattableText;
use kas::WindowId;

#[derive(Copy, Clone)]
struct MessageBoxOk;

impl_scope! {
    /// A simple message box.
    #[derive(Clone, Debug)]
    #[widget{
        layout = column: *;
    }]
    pub struct MessageBox<T: FormattableText + 'static> {
        #[widget_core]
        core: CoreData,
        title: String,
        #[widget]
        label: Label<T>,
        #[widget]
        button: TextButton,
    }

    impl Self {
        pub fn new<A: ToString>(title: A, message: T) -> Self {
            MessageBox {
                core: Default::default(),
                title: title.to_string(),
                label: Label::new(message),
                button: TextButton::new_msg("Ok", MessageBoxOk).with_keys(&[
                    VirtualKeyCode::Return,
                    VirtualKeyCode::Space,
                    VirtualKeyCode::NumpadEnter,
                ]),
            }
        }
    }

    impl kas::event::Handler for Self {
        fn on_message(&mut self, mgr: &mut EventMgr, _: usize) -> Response {
            if let Some(_) = mgr.try_pop_msg::<MessageBoxOk>() {
                mgr.send_action(TkAction::CLOSE);
            }
            Response::Unused
        }
    }

    impl kas::WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.enable_alt_bypass(self.id_ref(), true);
        }
    }

    impl kas::Window for Self {
        fn title(&self) -> &str {
            &self.title
        }

        fn icon(&self) -> Option<kas::Icon> {
            None // TODO
        }

        fn restrict_dimensions(&self) -> (bool, bool) {
            (true, true)
        }

        // do not support overlays (yet?)
        fn add_popup(&mut self, _: &mut EventMgr, _: WindowId, _: kas::Popup) {
            panic!("MessageBox does not (currently) support pop-ups");
        }

        fn remove_popup(&mut self, _: &mut EventMgr, _: WindowId) {}
        fn resize_popups(&mut self, _: &mut SetRectMgr) {}
    }
}
