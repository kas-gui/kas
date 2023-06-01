// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.
//!
//! # Design status
//!
//! At the current time, only a minimal selection of dialog boxes are provided
//! and their design is likely to change.

use crate::{EditBox, Filler, Label, TextButton};
use kas::event::{Command, VirtualKeyCode};
use kas::prelude::*;
use kas::text::format::FormattableText;
use kas::{Icon, Widget};
use std::borrow::Cow;

impl_scope! {
    /// A simple window around a widget
    #[autoimpl(Debug ignore self.icon, self.config_fn)]
    #[widget(layout = self.inner;)]
    pub struct Window<W: Widget<Data = ()>> {
        core: widget_core!(),
        restrict_dimensions: (bool, bool),
        title: String,
        #[widget]
        inner: W,
        icon: Option<Icon>,
        config_fn: Option<Box<dyn Fn(&Self, &mut ConfigCx<W::Data>)>>,
    }

    impl Widget for Self {
        fn configure(&mut self, cx: &mut ConfigCx<W::Data>) {
            if let Some(f) = self.config_fn.as_ref() {
                f(self, cx);
            }
        }
    }

    impl kas::Window for Self {
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

impl<W: Widget<Data = ()>> Window<W> {
    /// Construct
    pub fn new<T: ToString>(title: T, inner: W) -> Window<W> {
        Window {
            core: Default::default(),
            restrict_dimensions: (true, false),
            title: title.to_string(),
            inner,
            icon: None,
            config_fn: None,
        }
    }

    /// Set a closure to be called on initialisation
    ///
    /// This closure is called before sizing, drawing and event handling.
    /// It may be called more than once.
    pub fn on_configure(
        mut self,
        config_fn: impl Fn(&Self, &mut ConfigCx<W::Data>) + 'static,
    ) -> Self {
        self.config_fn = Some(Box::new(config_fn));
        self
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
    #[derive(Debug)]
    #[widget{
        layout = column! [self.label, self.button];
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
        pub fn new<I: ToString>(title: I, message: T) -> Self {
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
        fn handle_messages(&mut self, mgr: &mut EventCx<()>) {
            if let Some(MessageBoxOk) = mgr.try_pop() {
                mgr.send_action(Action::CLOSE);
            }
        }

        fn configure(&mut self, mgr: &mut ConfigCx<()>) {
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

/// Message sent by [`TextEdit`] on closure.
#[derive(Debug)]
pub enum TextEditResult {
    Cancel,
    Ok(String),
}

#[derive(Clone, Debug)]
struct MsgClose(bool);

impl_scope! {
    #[derive(Debug)]
    #[widget{
        layout = grid! {
            (0..3, 0) => self.edit,
            (0, 1) => Filler::maximize(),
            (1, 1) => TextButton::new_msg("&Cancel", MsgClose(false)),
            (2, 1) => TextButton::new_msg("&Save", MsgClose(true)),
        };
    }]
    /// An editor over a `String`
    ///
    /// Emits a [`TextEditResult`] message on closure.
    pub struct TextEdit {
        core: widget_core!(),
        title: Cow<'static, str>,
        #[widget]
        edit: EditBox,
    }

    impl Self {
        /// Construct
        pub fn new(
            title: impl Into<Cow<'static, str>>,
            text: impl ToString,
            multi_line: bool,
        ) -> Self {
            TextEdit {
                core: Default::default(),
                title: title.into(),
                edit: EditBox::text(text).with_multi_line(multi_line),
            }
        }

        fn close(&mut self, cx: &mut EventCx<()>, commit: bool) -> Response {
            cx.push(if commit {
                TextEditResult::Ok(self.edit.get_string())
            } else {
                TextEditResult::Cancel
            });
            Response::Used
        }
    }

    impl Widget for Self {
        fn configure(&mut self, mgr: &mut ConfigCx<()>) {
            mgr.register_nav_fallback(self.id());

            // Focus first item initially:
            if mgr.nav_focus().is_none() {
                mgr.next_nav_focus(self.id(), false, true);
            }
        }

        fn handle_event(&mut self, mgr: &mut EventCx<()>, event: Event) -> Response {
            match event {
                Event::Command(Command::Escape) => self.close(mgr, false),
                Event::Command(Command::Enter) => self.close(mgr, true),
                _ => Response::Unused,
            }
        }

        fn handle_messages(&mut self, mgr: &mut EventCx<()>) {
            if let Some(MsgClose(commit)) = mgr.try_pop() {
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
