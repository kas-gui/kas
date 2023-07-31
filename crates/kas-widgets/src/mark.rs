// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Mark widget

use kas::prelude::*;
use kas::theme::MarkStyle;
use std::fmt::Debug;

impl_scope! {
    /// A mark
    ///
    /// These are small theme-defined "glyphs"; see [`MarkStyle`]. They may be
    /// used as icons or visual connectors. See also [`MarkButton`].
    ///
    /// TODO: expand or replace.
    #[derive(Clone, Debug)]
    #[widget {
        Data = ();
    }]
    pub struct Mark {
        core: widget_core!(),
        style: MarkStyle,
    }
    impl Self {
        /// Construct
        pub fn new(style: MarkStyle) -> Self {
            Mark {
                core: Default::default(),
                style,
            }
        }

        /// Get mark style
        #[inline]
        pub fn mark(&self) -> MarkStyle {
            self.style
        }

        /// Set mark style
        #[inline]
        pub fn set_mark(&mut self, mark: MarkStyle) {
            self.style = mark;
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
}

impl_scope! {
    /// A mark which is also a button
    ///
    /// A clickable button over a [`Mark`].
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
