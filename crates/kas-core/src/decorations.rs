// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Title bar
//!
//! Note: due to definition in kas-core, some widgets must be duplicated.

use crate::event::{CursorIcon, ResizeDirection};
use crate::text::{NotReady, Text};
use crate::theme::TextClass;
use kas::prelude::*;
use kas::theme::MarkStyle;
use kas_macros::impl_scope;
use std::fmt::Debug;

/// Available decoration modes
///
/// See [`Window::decorations`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Decorations {
    /// No decorations
    ///
    /// The root widget is drawn as a simple rectangle with no borders.
    None,
    /// Add a simple themed border to the widget
    ///
    /// Probably looks better if [`Window::transparent`] is true.
    Border,
    /// Toolkit-drawn decorations
    ///
    /// Decorations will match the toolkit theme, not the platform theme.
    /// These decorations may not have all the same capabilities.
    ///
    /// Probably looks better if [`Window::transparent`] is true.
    Toolkit,
    /// Server-side decorations
    ///
    /// Decorations are drawn by the window manager, if available.
    Server,
}

impl_scope! {
    /// A border region
    ///
    /// Does not draw anything; used solely for event handling.
    #[widget {
        cursor_icon = self.cursor_icon();
    }]
    pub(crate) struct Border {
        core: widget_core!(),
        resizable: bool,
        direction: ResizeDirection,
    }

    impl Self {
        pub fn new(direction: ResizeDirection) -> Self {
            Border {
                core: Default::default(),
                resizable: true,
                direction,
            }
        }

        pub fn set_resizable(&mut self, resizable: bool) {
            self.resizable = resizable;
        }

        fn cursor_icon(&self) -> CursorIcon {
            if self.resizable {
                self.direction.into()
            } else {
                CursorIcon::default()
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, _: SizeCx, _axis: AxisInfo) -> SizeRules {
            SizeRules::EMPTY
        }

        fn draw(&mut self, _: DrawCx) {}
    }

    impl Events for Self {
        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::PressStart { .. } => {
                    cx.drag_resize_window(self.direction);
                    Used
                }
                _ => Unused,
            }
        }
    }
}

impl_scope! {
    /// A simple label
    #[derive(Clone, Debug, Default)]
    #[widget {
        Data = ();
    }]
    pub struct Label {
        core: widget_core!(),
        label: Text<String>,
    }

    impl Self {
        /// Construct from `label`
        #[inline]
        fn new(label: impl ToString) -> Self {
            Label {
                core: Default::default(),
                label: Text::new(label.to_string()),
            }
        }

        /// Text class
        pub const CLASS: TextClass = TextClass::Label(false);
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            axis.set_default_align_hv(Align::Center, Align::Center);
            sizer.text_rules(&mut self.label, Self::CLASS, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            cx.text_set_size(&mut self.label, Self::CLASS, rect.size, None);
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.text(self.rect(), &self.label, Self::CLASS);
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }

    impl HasString for Self {
        fn set_string(&mut self, string: String) -> Action {
            self.label.set_string(string);
            match self.label.prepare() {
                Err(NotReady) => Action::empty(),
                Ok(false) => Action::REDRAW,
                Ok(true) => Action::RESIZE,
            }
        }
    }
}

impl_scope! {
    /// A mark which is also a button
    ///
    /// This button is not keyboard navigable; only mouse/touch interactive.
    ///
    /// Uses stretch policy [`Stretch::Low`].
    #[derive(Clone, Debug)]
    #[widget {
        hover_highlight = true;
    }]
    pub struct MarkButton<M: Clone + Debug + 'static> {
        core: widget_core!(),
        style: MarkStyle,
        msg: M,
    }

    impl Self {
        /// Construct
        ///
        /// A clone of `msg` is sent as a message on click.
        pub fn new(style: MarkStyle, msg: M) -> Self {
            MarkButton {
                core: Default::default(),
                style,
                msg,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            sizer.feature(self.style.into(), axis)
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.mark(self.core.rect, self.style);
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            event.on_activate(cx, self.id(), |cx| {
                cx.push(self.msg.clone());
                Used
            })
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum TitleBarButton {
    Minimize,
    Maximize,
    Close,
}

impl_scope! {
    /// A window's title bar (part of decoration)
    #[derive(Clone, Default)]
    #[widget{
        layout = row! [
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
        title: Label,
    }

    impl Self {
        /// Construct a title bar
        #[inline]
        pub fn new(title: impl ToString) -> Self {
            TitleBar {
                core: Default::default(),
                title: Label::new(title),
            }
        }

        /// Get the title
        pub fn title(&self) -> &str {
            self.title.label.as_str()
        }

        /// Set the title
        pub fn set_title(&mut self, title: String) -> Action {
            self.title.set_string(title)
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::PressStart { .. } => {
                    cx.drag_window();
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(msg) = cx.try_pop() {
                match msg {
                    TitleBarButton::Minimize => {
                        #[cfg(winit)]
                        if let Some(w) = cx.winit_window() {
                            w.set_minimized(true);
                        }
                    }
                    TitleBarButton::Maximize => {
                        #[cfg(winit)]
                        if let Some(w) = cx.winit_window() {
                            w.set_maximized(!w.is_maximized());
                        }
                    }
                    TitleBarButton::Close => cx.action(self, Action::CLOSE),
                }
            }
        }
    }
}
