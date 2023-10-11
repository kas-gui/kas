// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu Entries

use super::{Menu, SubItems};
use crate::{AccessLabel, CheckBox};
use kas::theme::{FrameStyle, TextClass};
use kas::{layout, prelude::*};
use std::fmt::Debug;

impl_scope! {
    /// A standard menu entry
    ///
    /// # Messages
    ///
    /// A `MenuEntry` has an associated message value of type `M`. A clone of
    /// this value is pushed when the entry is activated.
    #[derive(Clone, Debug, Default)]
    #[widget {
        layout = self.label;
        navigable = true;
    }]
    pub struct MenuEntry<M: Clone + Debug + 'static> {
        core: widget_core!(),
        #[widget]
        label: AccessLabel,
        msg: M,
    }

    impl Layout for Self {
        fn find_id(&mut self, coord: Coord) -> Option<Id> {
            self.rect().contains(coord).then(|| self.id())
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.frame(self.rect(), FrameStyle::MenuEntry, Default::default());
            self.label.draw(draw);
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
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.get_str()
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, code) if cmd.is_activate() => {
                    cx.push(self.msg.clone());
                    if let Some(code) = code {
                        cx.depress_with_key(self.id(), code);
                    }
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(kas::message::Activate(code)) = cx.try_pop() {
                cx.push(self.msg.clone());
                if let Some(code) = code {
                    cx.depress_with_key(self.id(), code);
                }
            }
        }
    }

    impl Menu for Self {
        fn sub_items(&mut self) -> Option<SubItems> {
            Some(SubItems {
                label: Some(&mut self.label),
                ..Default::default()
            })
        }
    }

    impl PartialEq<M> for Self where M: PartialEq {
        #[inline]
        fn eq(&self, rhs: &M) -> bool {
            self.msg == *rhs
        }
    }
}

impl_scope! {
    /// A menu entry which can be toggled
    #[widget {
        layout = row! [self.checkbox, self.label];
    }]
    pub struct MenuToggle<A> {
        core: widget_core!(),
        #[widget]
        checkbox: CheckBox<A>,
        #[widget(&())]
        label: AccessLabel,
    }

    impl Layout for Self {
        fn find_id(&mut self, coord: Coord) -> Option<Id> {
            self.rect().contains(coord).then(|| self.checkbox.id())
        }

        fn draw(&mut self, mut draw: DrawCx) {
            let mut draw = draw.re_id(self.checkbox.id());
            draw.frame(self.rect(), FrameStyle::MenuEntry, Default::default());
            <Self as layout::AutoLayout>::draw(self, draw);
        }
    }

    impl Events for Self {
        type Data = A;

        fn handle_messages(&mut self, cx: &mut EventCx, data: &Self::Data) {
            if let Some(kas::message::Activate(code)) = cx.try_pop() {
                self.checkbox.toggle(cx, data);
                if let Some(code) = code {
                    cx.depress_with_key(self.id(), code);
                }
            }
        }
    }

    impl Menu for Self {
        fn sub_items(&mut self) -> Option<SubItems> {
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
