// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu Entries

use super::{Menu, SubItems};
use crate::CheckBoxBare;
use kas::component::{Component, Label};
use kas::theme::{FrameStyle, IdRect, TextClass};
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
    #[widget]
    pub struct MenuEntry<M: Clone + Debug + 'static> {
        #[widget_core]
        core: kas::CoreData,
        label: Label<AccelString>,
        msg: M,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.id_ref(), self.label.keys());
        }

        fn key_nav(&self) -> bool {
            true
        }
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            layout::Layout::component(&mut self.label)
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.frame(&*self, FrameStyle::MenuEntry, Default::default());
            self.label.draw(draw, &self.core.id);
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
                label: Label::new(label.into(), TextClass::MenuLabel),
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
            self.label.as_str()
        }
    }

    impl SetAccel for Self {
        fn set_accel_string(&mut self, string: AccelString) -> TkAction {
            let mut action = TkAction::empty();
            if self.label.keys() != string.keys() {
                action |= TkAction::RECONFIGURE;
            }
            let avail = self.core.rect.size;
            action | self.label.set_text_and_prepare(string, avail)
        }
    }

    impl Handler for Self {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Activate => {
                    mgr.push_msg(self.msg.clone());
                    Response::Used
                }
                _ => Response::Unused,
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
}

impl_scope! {
    /// A menu entry which can be toggled
    #[autoimpl(Debug)]
    #[autoimpl(HasBool using self.checkbox)]
    #[derive(Clone, Default)]
    #[widget]
    pub struct MenuToggle {
        #[widget_core]
        core: CoreData,
        #[widget]
        checkbox: CheckBoxBare,
        label: Label<AccelString>,
        layout_list: layout::DynRowStorage,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.checkbox.id_ref(), self.label.keys());
        }
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            let list = [
                layout::Layout::single(&mut self.checkbox),
                layout::Layout::component(&mut self.label),
            ];
            layout::Layout::list(list.into_iter(), Direction::Right, &mut self.layout_list)
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            Some(self.checkbox.id())
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let id = self.checkbox.id();
            draw.frame(IdRect(&id, self.rect()), FrameStyle::MenuEntry, Default::default());
            self.layout().draw(draw, &id);
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

    impl MenuToggle {
        /// Construct a toggleable menu entry with a given `label`
        #[inline]
        pub fn new<T: Into<AccelString>>(label: T) -> Self {
            MenuToggle {
                core: Default::default(),
                checkbox: CheckBoxBare::new(),
                label: Label::new(label.into(), TextClass::MenuLabel),
                layout_list: Default::default(),
            }
        }

        /// Set event handler `f`
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        #[must_use]
        pub fn on_toggle<F>(self, f: F) -> Self
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
             MenuToggle {
                core: self.core,
                checkbox: self.checkbox.on_toggle(f),
                label: self.label,
                layout_list: self.layout_list,
            }
        }
    }

    impl Self {
        /// Construct a toggleable menu entry with a given `label` and event handler `f`
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        pub fn new_on<T: Into<AccelString>, F>(label: T, f: F) -> Self
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
            MenuToggle::new(label).on_toggle(f)
        }

        /// Set the initial state of the checkbox.
        #[inline]
        #[must_use]
        pub fn with_state(mut self, state: bool) -> Self {
            self.checkbox = self.checkbox.with_state(state);
            self
        }
    }
}
