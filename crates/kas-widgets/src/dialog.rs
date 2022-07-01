// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.

use crate::{EditBox, Filler, Label, TextButton};
use kas::event::{Command, VirtualKeyCode};
use kas::model::{SharedRc, SingleData};
use kas::prelude::*;
use kas::text::format::FormattableText;
use kas::{Icon, Widget};
use std::borrow::Cow;

impl_scope! {
    /// A simple window around a widget
    #[autoimpl(Clone where W: Clone)]
    #[autoimpl(Debug ignore self.icon)]
    #[widget(layout = self.inner;)]
    pub struct Window<W: Widget> {
        core: widget_core!(),
        restrict_dimensions: (bool, bool),
        title: String,
        #[widget]
        inner: W,
        icon: Option<Icon>,
    }

    impl<W: Widget> kas::Window for Window<W> {
        fn title(&self) -> &str {
            &self.title
        }

        fn icon(&self) -> Option<Icon> {
            self.icon.clone()
        }

        fn restrict_dimensions(&self) -> (bool, bool) {
            self.restrict_dimensions
        }
    }
}

impl<W: Widget> Window<W> {
    /// Construct
    pub fn new<T: ToString>(title: T, inner: W) -> Window<W> {
        Window {
            core: Default::default(),
            restrict_dimensions: (true, false),
            title: title.to_string(),
            inner,
            icon: None,
        }
    }

    /// Configure whether min/max dimensions are forced
    ///
    /// By default, the min size is enforced but not the max.
    pub fn set_restrict_dimensions(&mut self, min: bool, max: bool) {
        self.restrict_dimensions = (min, max);
    }

    /// Set the window icon
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon;
    }
}

#[derive(Copy, Clone, Debug)]
struct MessageBoxOk;

impl_scope! {
    /// A simple message box.
    #[derive(Clone, Debug)]
    #[widget{
        layout = column: [self.label, self.button];
    }]
    pub struct MessageBox<T: FormattableText + 'static> {
        core: widget_core!(),
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

    impl Widget for Self {
        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(MessageBoxOk) = mgr.try_pop_msg() {
                mgr.send_action(TkAction::CLOSE);
            }
        }

        fn configure(&mut self, mgr: &mut ConfigMgr) {
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
    }
}

#[derive(Clone, Debug)]
struct MsgClose(bool);

impl_scope! {
    #[derive(Debug)]
    #[widget{
        layout = grid: {
            0..3, 0: self.edit;
            0, 1: Filler::maximize();
            1, 1: TextButton::new_msg("&Cancel", MsgClose(false));
            2, 1: TextButton::new_msg("&Save", MsgClose(true));
        };
    }]
    /// An editor over a shared `String`
    pub struct TextEdit<T: SingleData<Item = String> = SharedRc<String>> {
        core: widget_core!(),
        title: Cow<'static, str>,
        data: T,
        #[widget]
        edit: EditBox,
    }

    impl Self {
        /// Construct
        pub fn new(title: impl Into<Cow<'static, str>>, multi_line: bool, data: T) -> Self {
            let text = data.get_cloned();
            TextEdit {
                core: Default::default(),
                title: title.into(),
                data,
                edit: EditBox::new(text).multi_line(multi_line),
            }
        }

        fn close(&mut self, mgr: &mut EventMgr, commit: bool) -> Response {
            if commit {
                self.data.update(mgr, self.edit.get_string());
            }
            mgr.send_action(TkAction::CLOSE);
            Response::Used
        }
    }

    impl Widget for Self {
        fn configure(&mut self, mgr: &mut ConfigMgr) {
            mgr.register_nav_fallback(self.id());

            // Focus first item initially:
            if mgr.nav_focus().is_none() {
                mgr.next_nav_focus(self, false, true);
            }
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Command(Command::Escape) => self.close(mgr, false),
                Event::Command(Command::Return) => self.close(mgr, true),
                _ => Response::Unused,
            }
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(MsgClose(commit)) = mgr.try_pop_msg() {
                let _ = self.close(mgr, commit);
            }
        }
    }

    impl kas::Window for Self {
        fn title(&self) -> &str {
            &self.title
        }
    }
}
