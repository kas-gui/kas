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
use crate::theme::{FrameStyle, MarkStyle};
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
        fn size_rules(&mut self, _: &mut SizeCx, _axis: AxisInfo) -> SizeRules {
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
                Event::PressStart(press) => press.drag_resize_window(cx, self.direction),
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
    #[derive(Default)]
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
                let mut window = cx.top_window();
                match msg {
                    TitleBarButton::Minimize => window.set_minimized(true),
                    TitleBarButton::Maximize => {
                        let maximize = !window.is_maximized();
                        window.set_maximized(maximize)
                    }
                    TitleBarButton::Close => window.close(),
                }
            }
        }
    }
}

#[impl_self]
mod TitleBar {
    /// A window's title bar (part of decoration)
    ///
    /// This widget has no external margin.
    #[widget]
    #[layout(frame!(row! [
        self.title.align(AlignHints::CENTER).with_stretch(Stretch::Maximize, Stretch::None),
        self.buttons,
    ]).with_style(FrameStyle::None))]
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

        /// Get the cached title
        pub(crate) fn title(&self) -> &str {
            self.title.as_str()
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

        fn update(&mut self, cx: &mut ConfigCx, _: &Self::Data) {
            // NOTE: this if is used to avoid overwriting the title passed to
            // new() when a title has not been set through set_title().
            if !cx.window_title.is_empty() {
                self.title.set_str_from_cx(cx, |cx| &cx.window_title)
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::PressStart(press) => press.drag_window(cx),
                _ => Unused,
            }
        }
    }
}
