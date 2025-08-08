// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window title-bar and border decorations
//!
//! Note: due to definition in kas-core, some widgets must be duplicated.

use super::{Label, MarkButton};
use crate::event::CursorIcon;
use crate::prelude::*;
use crate::theme::MarkStyle;
use crate::window::ResizeDirection;
use kas_macros::impl_self;
use std::fmt::Debug;

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

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Border
        }
    }

    impl Events for Self {
        type Data = ();

        fn mouse_over_icon(&self) -> Option<CursorIcon> {
            if self.resizable {
                Some(self.direction.into())
            } else {
                None
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::PressStart(_) => {
                    cx.drag_resize_window(self.direction);
                    Used
                }
                _ => Unused,
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
        MarkButton::new_msg(MarkStyle::Chevron(Direction::Down), "Minimize", TitleBarButton::Minimize),
        MarkButton::new_msg(MarkStyle::Chevron(Direction::Up), "Maximize", TitleBarButton::Maximize),
        MarkButton::new_msg(MarkStyle::X, "Close", TitleBarButton::Close),
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
    #[layout(row! [self.title.align(AlignHints::CENTER), self.buttons])]
    pub struct TitleBar {
        core: widget_core!(),
        #[widget]
        title: Label<String>,
        #[widget]
        buttons: TitleBarButtons,
    }

    impl Self {
        /// Construct a title bar
        #[inline]
        pub fn new(title: impl ToString) -> Self {
            TitleBar {
                core: Default::default(),
                title: Label::new(title.to_string()),
                buttons: Default::default(),
            }
        }

        /// Get the title
        pub fn title(&self) -> &str {
            self.title.as_str()
        }

        /// Set the title
        pub fn set_title(&mut self, cx: &mut EventState, title: String) {
            self.title.set_string(cx, title)
        }
    }

    impl Tile for Self {
        fn role(&self, cx: &mut dyn RoleCx) -> Role<'_> {
            cx.set_label(self.title.id());
            Role::TitleBar
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::PressStart(_) => {
                    cx.drag_window();
                    Used
                }
                _ => Unused,
            }
        }
    }
}
