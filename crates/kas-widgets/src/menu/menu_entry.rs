// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu Entries

use super::{Menu, SubItems};
use crate::{AccelLabel, CheckBox};
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
        label: AccelLabel,
        msg: M,
    }

    impl Layout for Self {
        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.rect().contains(coord).then(|| self.id())
        }

        fn draw(&mut self, mut draw: DrawMgr) {
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
        pub fn new<S: Into<AccelString>>(label: S, msg: M) -> Self {
            MenuEntry {
                core: Default::default(),
                label: AccelLabel::new(label).with_class(TextClass::MenuLabel),
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

    impl SetAccel for Self {
        #[inline]
        fn set_accel_string(&mut self, string: AccelString) -> Action {
            self.label.set_accel_string(string)
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_event(&mut self, _: &Self::Data, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Command(cmd) if cmd.is_activate() => {
                    mgr.push(self.msg.clone());
                    Response::Used
                }
                _ => Response::Unused,
            }
        }

        fn handle_messages(&mut self, _: &Self::Data, mgr: &mut EventMgr) {
            if let Some(kas::message::Activate) = mgr.try_pop() {
                mgr.push(self.msg.clone());
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
        Data = A;
        layout = row! [self.checkbox, self.label];
    }]
    pub struct MenuToggle<A> {
        core: widget_core!(),
        #[widget]
        checkbox: CheckBox<A>,
        #[widget(&())]
        label: AccelLabel,
    }

    impl Layout for Self {
        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.rect().contains(coord).then(|| self.checkbox.id())
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let mut draw = draw.re_id(self.checkbox.id());
            draw.frame(self.rect(), FrameStyle::MenuEntry, Default::default());
            <Self as layout::AutoLayout>::draw(self, draw);
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
            label: impl Into<AccelString>,
            state_fn: impl Fn(&ConfigMgr, &A) -> bool + 'static,
        ) -> Self {
            MenuToggle {
                core: Default::default(),
                checkbox: CheckBox::new(state_fn),
                label: AccelLabel::new(label).with_class(TextClass::MenuLabel),
            }
        }

        /// Set the "toggle" handler
        ///
        /// When the check box is set or unset, the closure `on_toggle` is called.
        #[inline]
        #[must_use]
        pub fn on_toggle<F>(self, on_toggle: F) -> Self
        where
            F: Fn(&mut EventMgr, &A, bool) + 'static,
        {
            MenuToggle {
                core: self.core,
                checkbox: self.checkbox.on_toggle(on_toggle),
                label: self.label,
            }
        }

        /// Construct a toggleable menu entry with the given `label` and `message_fn`
        ///
        /// - `label` is displayed to the left or right (according to text direction)
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `message_fn` is emitted when toggled
        #[inline]
        pub fn new_msg<M: Debug + 'static>(
            label: impl Into<AccelString>,
            state_fn: impl Fn(&ConfigMgr, &A) -> bool + 'static,
            message_fn: impl Fn(bool) -> M + 'static,
        ) -> Self {
            MenuToggle::new(label, state_fn)
                .on_toggle(move |cx, _, state| cx.push(message_fn(state)))
        }
    }
}
