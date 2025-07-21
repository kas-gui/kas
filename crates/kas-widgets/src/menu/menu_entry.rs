// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu Entries

use super::{Menu, SubItems};
use crate::{AccessLabel, CheckBox};
use kas::prelude::*;
use kas::theme::{FrameStyle, TextClass};
use std::fmt::Debug;

#[impl_self]
mod MenuEntry {
    /// A standard menu entry
    ///
    /// # Messages
    ///
    /// A `MenuEntry` has an associated message value of type `M`. A clone of
    /// this value is pushed when the entry is activated.
    ///
    /// # Messages
    ///
    /// [`kas::messages::Activate`] may be used to trigger the menu entry.
    #[derive(Clone, Debug, Default)]
    #[widget]
    #[layout(self.label)]
    pub struct MenuEntry<M: Clone + Debug + 'static> {
        core: widget_core!(),
        #[widget]
        label: AccessLabel,
        msg: M,
    }

    impl Layout for Self {
        fn draw(&self, mut draw: DrawCx) {
            draw.frame(self.rect(), FrameStyle::MenuEntry, Default::default());
            self.label.draw(draw.re());
        }
    }

    impl Tile for Self {
        fn navigable(&self) -> bool {
            true
        }

        fn role(&self, cx: &mut dyn RoleCx) -> Role<'_> {
            cx.set_label(self.label.id());
            Role::Button
        }

        fn probe(&self, _: Coord) -> Id {
            self.id()
        }
    }

    impl Self {
        /// Construct a menu item with a given `label` and `msg`
        ///
        /// The message `msg` is emitted on activation. Any
        /// type supporting `Clone` is valid, though it is recommended to use a
        /// simple `Copy` type (e.g. an enum).
        pub fn new_msg<S: Into<AccessString>>(label: S, msg: M) -> Self {
            MenuEntry {
                core: Default::default(),
                label: AccessLabel::new(label).with_class(TextClass::MenuLabel),
                msg,
            }
        }

        /// Replace the message value
        pub fn set_msg(&mut self, msg: M) {
            self.msg = msg;
        }

        /// Get text contents
        pub fn as_str(&self) -> &str {
            self.label.as_str()
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, code) if cmd.is_activate() => {
                    cx.push(self.msg.clone());
                    cx.depress_with_key(self.id(), code);
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(kas::messages::Activate(code)) = cx.try_pop() {
                cx.push(self.msg.clone());
                cx.depress_with_key(self.id(), code);
            }
        }
    }

    impl Menu for Self {
        fn sub_items(&mut self) -> Option<SubItems<'_>> {
            Some(SubItems {
                label: Some(&mut self.label),
                ..Default::default()
            })
        }
    }

    impl PartialEq<M> for Self
    where
        M: PartialEq,
    {
        #[inline]
        fn eq(&self, rhs: &M) -> bool {
            self.msg == *rhs
        }
    }
}

#[impl_self]
mod MenuToggle {
    /// A menu entry which can be toggled
    ///
    /// # Messages
    ///
    /// [`kas::messages::Activate`] may be used to toggle the menu entry.
    #[widget]
    #[layout(row! [self.checkbox, self.label])]
    pub struct MenuToggle<A> {
        core: widget_core!(),
        #[widget]
        checkbox: CheckBox<A>,
        #[widget(&())]
        label: AccessLabel,
    }

    impl Layout for Self {
        fn draw(&self, mut draw: DrawCx) {
            draw.set_id(self.checkbox.id());
            draw.frame(self.rect(), FrameStyle::MenuEntry, Default::default());
            kas::MacroDefinedLayout::draw(self, draw);
        }
    }

    impl Tile for Self {
        fn role_child_properties(&self, cx: &mut dyn RoleCx, index: usize) {
            if index == widget_index!(self.checkbox) {
                cx.set_label(self.label.id());
            }
        }

        fn probe(&self, _: Coord) -> Id {
            self.checkbox.id()
        }
    }

    impl Events for Self {
        type Data = A;

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            let id = self.make_child_id(widget_index!(self.checkbox));
            if id.is_valid() {
                cx.configure(self.checkbox.as_node(data), id);

                if let Some(key) = self.label.access_key() {
                    cx.add_access_key(self.checkbox.id_ref(), key.clone());
                }
            }

            let id = self.make_child_id(widget_index!(self.label));
            if id.is_valid() {
                cx.configure(self.label.as_node(&()), id);
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &Self::Data) {
            if let Some(kas::messages::Activate(code)) = cx.try_pop() {
                self.checkbox.toggle(cx, data);
                cx.depress_with_key(self.id(), code);
            }
        }
    }

    impl Menu for Self {
        fn sub_items(&mut self) -> Option<SubItems<'_>> {
            Some(SubItems {
                label: Some(&mut self.label),
                toggle: Some(&mut self.checkbox),
                ..Default::default()
            })
        }
    }

    impl Self {
        /// Construct a toggleable menu entry with the given `label`
        ///
        /// - `label` is displayed to the left or right (according to text direction)
        /// - `state_fn` extracts the current state from input data
        #[inline]
        pub fn new(
            label: impl Into<AccessString>,
            state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static,
        ) -> Self {
            MenuToggle {
                core: Default::default(),
                checkbox: CheckBox::new(state_fn),
                label: AccessLabel::new(label).with_class(TextClass::MenuLabel),
            }
        }

        /// Call the handler `f` on toggle
        #[inline]
        #[must_use]
        pub fn with<F>(self, f: F) -> Self
        where
            F: Fn(&mut EventCx, &A, bool) + 'static,
        {
            MenuToggle {
                core: self.core,
                checkbox: self.checkbox.with(f),
                label: self.label,
            }
        }

        /// Send the message generated by `f` on toggle
        #[inline]
        #[must_use]
        pub fn with_msg<M>(self, f: impl Fn(bool) -> M + 'static) -> Self
        where
            M: std::fmt::Debug + 'static,
        {
            self.with(move |cx, _, state| cx.push(f(state)))
        }

        /// Construct a toggleable menu entry with the given `label` and `msg_fn`
        ///
        /// - `label` is displayed to the left or right (according to text direction)
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `msg_fn` is emitted when toggled
        #[inline]
        pub fn new_msg<M: Debug + 'static>(
            label: impl Into<AccessString>,
            state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static,
            msg_fn: impl Fn(bool) -> M + 'static,
        ) -> Self {
            MenuToggle::new(label, state_fn).with_msg(msg_fn)
        }
    }
}
