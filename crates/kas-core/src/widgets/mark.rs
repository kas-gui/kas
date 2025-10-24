// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Mark widget

use kas::prelude::*;
use kas::theme::MarkStyle;
use std::fmt::Debug;

#[impl_self]
mod Mark {
    /// A mark
    ///
    /// These are small theme-defined "glyphs"; see [`MarkStyle`]. They may be
    /// used as icons or visual connectors. See also [`MarkButton`].
    ///
    /// TODO: expand or replace.
    #[derive(Clone, Debug)]
    #[widget]
    pub struct Mark {
        core: widget_core!(),
        style: MarkStyle,
        label: String,
    }
    impl Self {
        /// Construct
        pub fn new(style: MarkStyle, label: impl ToString) -> Self {
            Mark {
                core: Default::default(),
                style,
                label: label.to_string(),
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
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            cx.feature(self.style.into(), axis)
        }

        fn draw(&self, mut draw: DrawCx) {
            draw.mark(self.rect(), self.style);
        }
    }

    impl Tile for Self {
        fn tooltip(&self) -> Option<&str> {
            Some(&self.label)
        }

        fn role(&self, cx: &mut dyn RoleCx) -> Role<'_> {
            cx.set_label(&self.label);
            Role::Indicator
        }
    }
}

#[impl_self]
mod MarkButton {
    /// A mark which is also a button
    ///
    /// A clickable button over a [`Mark`].
    /// This button is not keyboard navigable; only mouse/touch interactive.
    ///
    /// Uses stretch policy [`Stretch::Low`].
    ///
    /// # Messages
    ///
    /// [`kas::messages::Activate`] may be used to trigger the button.
    #[derive(Clone, Debug)]
    #[widget]
    pub struct MarkButton<M: Clone + Debug + 'static> {
        core: widget_core!(),
        style: MarkStyle,
        label: String,
        msg: M,
    }

    impl Self {
        /// Construct
        ///
        /// A clone of `msg` is sent as a message on click.
        pub fn new_msg(style: MarkStyle, label: impl ToString, msg: M) -> Self {
            MarkButton {
                core: Default::default(),
                style,
                label: label.to_string(),
                msg,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            cx.feature(self.style.into(), axis)
                .with_stretch(Stretch::Low)
        }

        fn draw(&self, mut draw: DrawCx) {
            draw.mark(self.rect(), self.style);
        }
    }

    impl Tile for Self {
        fn tooltip(&self) -> Option<&str> {
            Some(&self.label)
        }

        fn role(&self, cx: &mut dyn RoleCx) -> Role<'_> {
            cx.set_label(&self.label);
            Role::Button
        }
    }

    impl Events for Self {
        const REDRAW_ON_MOUSE_OVER: bool = true;

        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            event.on_click(cx, self.id(), |cx| cx.push(self.msg.clone()))
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(kas::messages::Activate(code)) = cx.try_pop() {
                cx.push(self.msg.clone());
                cx.depress_with_key(&self, code);
            }
        }
    }
}
