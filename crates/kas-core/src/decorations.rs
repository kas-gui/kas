// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window title-bar and border decorations
//!
//! Note: due to definition in kas-core, some widgets must be duplicated.

use crate::event::{CursorIcon, ResizeDirection};
use crate::prelude::*;
use crate::theme::MarkStyle;
use crate::theme::{Text, TextClass};
use kas_macros::impl_self;
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

#[impl_self]
mod Border {
    /// A border region
    ///
    /// Does not draw anything; used solely for event handling.
    #[widget]
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
    }

    impl Layout for Self {
        fn size_rules(&mut self, _: SizeCx, _axis: AxisInfo) -> SizeRules {
            SizeRules::EMPTY
        }

        fn draw(&self, _: DrawCx) {}
    }

    impl Events for Self {
        type Data = ();

        fn hover_icon(&self) -> Option<CursorIcon> {
            if self.resizable {
                Some(self.direction.into())
            } else {
                None
            }
        }

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

#[impl_self]
mod Label {
    /// A simple label
    #[derive(Clone, Debug, Default)]
    #[widget]
    #[layout(self.text)]
    pub(crate) struct Label {
        core: widget_core!(),
        text: Text<String>,
    }

    impl Self {
        /// Construct from `text`
        #[inline]
        fn new(text: impl ToString) -> Self {
            Label {
                core: Default::default(),
                text: Text::new(text.to_string(), TextClass::Label(false)),
            }
        }
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.text
                .set_rect(cx, rect, hints.combine(AlignHints::CENTER));
        }
    }

    impl Tile for Self {
        fn role(&self) -> Role<'_> {
            Role::Label(self.text.as_str())
        }
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.text);
        }
    }

    impl Self {
        /// Set text from a string
        pub fn set_string(&mut self, cx: &mut EventState, string: String) {
            if self.text.set_string(string) {
                cx.action(self.id(), self.text.reprepare_action());
            }
        }
    }
}

#[impl_self]
mod MarkButton {
    /// A mark which is also a button
    ///
    /// This button is not keyboard navigable; only mouse/touch interactive.
    ///
    /// Uses stretch policy [`Stretch::Low`].
    ///
    /// ### Messages
    ///
    /// [`kas::messages::Activate`] may be used to trigger the button.
    #[derive(Clone, Debug)]
    #[widget]
    pub(crate) struct MarkButton<M: Clone + Debug + 'static> {
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

        fn draw(&self, mut draw: DrawCx) {
            draw.mark(self.rect(), self.style);
        }
    }

    impl Tile for Self {
        fn role(&self) -> Role<'_> {
            Role::Button
        }
    }

    impl Events for Self {
        const REDRAW_ON_HOVER: bool = true;

        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            event.on_activate(cx, self.id(), |cx| {
                cx.push(self.msg.clone());
                Used
            })
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(kas::messages::Activate(code)) = cx.try_pop() {
                cx.push(self.msg.clone());
                cx.depress_with_key(self.id(), code);
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum TitleBarButton {
    Minimize,
    Maximize,
    Close,
}

#[impl_self]
mod TitleBarButtons {
    /// A set of title-bar buttons
    ///
    /// Currently, this consists of minimise, maximise and close buttons.
    #[derive(Clone, Default)]
    #[widget]
    #[layout(row! [
        MarkButton::new(MarkStyle::Point(Direction::Down), TitleBarButton::Minimize),
        MarkButton::new(MarkStyle::Point(Direction::Up), TitleBarButton::Maximize),
        MarkButton::new(MarkStyle::X, TitleBarButton::Close),
    ])]
    pub struct TitleBarButtons {
        core: widget_core!(),
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new() -> Self {
            TitleBarButtons {
                core: Default::default(),
            }
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(msg) = cx.try_pop() {
                match msg {
                    TitleBarButton::Minimize => {
                        if let Some(w) = cx.winit_window() {
                            w.set_minimized(true);
                        }
                    }
                    TitleBarButton::Maximize => {
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

#[impl_self]
mod TitleBar {
    /// A window's title bar (part of decoration)
    #[derive(Clone, Default)]
    #[widget]
    #[layout(row! [self.title, self.buttons])]
    pub struct TitleBar {
        core: widget_core!(),
        #[widget]
        title: Label,
        #[widget]
        buttons: TitleBarButtons,
    }

    impl Self {
        /// Construct a title bar
        #[inline]
        pub fn new(title: impl ToString) -> Self {
            TitleBar {
                core: Default::default(),
                title: Label::new(title),
                buttons: Default::default(),
            }
        }

        /// Get the title
        pub fn title(&self) -> &str {
            self.title.text.as_str()
        }

        /// Set the title
        pub fn set_title(&mut self, cx: &mut EventState, title: String) {
            self.title.set_string(cx, title)
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
    }
}
