// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Title bar

use super::Label;
use super::MarkButton;
use kas::prelude::*;
use kas::theme::MarkStyle;

#[derive(Copy, Clone, Debug)]
enum TitleBarButton {
    Minimize,
    Maximize,
    Close,
}

impl_scope! {
    /// A window's title bar (part of decoration)
    #[derive(Clone, Debug, Default)]
    #[widget{
        layout = row: [
            // self.icon,
            self.title,
            MarkButton::new(MarkStyle::Point(Direction::Down), TitleBarButton::Minimize),
            MarkButton::new(MarkStyle::Point(Direction::Up), TitleBarButton::Maximize),
            MarkButton::new(MarkStyle::X, TitleBarButton::Close),
        ];
    }]
    pub struct TitleBar {
        core: widget_core!(),
        #[widget]
        title: Label<String>,
    }

    impl Self {
        /// Construct a title bar
        #[inline]
        pub fn new(title: String) -> Self {
            TitleBar {
                core: Default::default(),
                title: Label::new(title),
            }
        }
    }

    impl Widget for Self {
        fn handle_message(&mut self, mgr: &mut EventMgr) {
            if let Some(msg) = mgr.try_pop() {
                match msg {
                    TitleBarButton::Minimize => todo!(),
                    TitleBarButton::Maximize => todo!(),
                    TitleBarButton::Close => mgr.send_action(Action::CLOSE),
                }
            }
        }
    }
}
