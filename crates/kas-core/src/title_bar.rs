// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Title bar
//!
//! Note: due to definition in kas-core, some widgets must be duplicated.

use crate::event::ConfigCx;
use crate::geom::Rect;
use crate::layout::{Align, AxisInfo, SizeRules};
use crate::text::Text;
use crate::theme::{DrawCx, SizeCx, TextClass};
use crate::Layout;
use kas::prelude::*;
use kas::theme::MarkStyle;
use kas_macros::impl_scope;
use std::fmt::Debug;

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

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> Response {
            event.on_activate(cx, self.id(), |cx| {
                cx.push(self.msg.clone());
                Response::Used
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
    }

    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(msg) = cx.try_pop() {
                match msg {
                    TitleBarButton::Minimize => {
                        #[cfg(features = "winit")]
                        if let Some(w) = cx.winit_window() {
                            // TODO: supported in winit 0.28:
                            // let is_minimized = w.is_minimized().unwrap_or(false);
                            let is_minimized = false;
                            w.set_minimized(!is_minimized);
                        }
                    }
                    TitleBarButton::Maximize => {
                        #[cfg(features = "winit")]
                        if let Some(w) = cx.winit_window() {
                            w.set_maximized(!w.is_maximized());
                        }
                    }
                    TitleBarButton::Close => cx.send_action(Action::CLOSE),
                }
            }
        }
    }
}
