// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu Entries

use super::Menu;
use crate::CheckBoxBare;
use kas::theme::{FrameStyle, TextClass};
use kas::{layout, prelude::*};
use std::fmt::Debug;

widget! {
    /// A standard menu entry
    #[derive(Clone, Debug, Default)]
    pub struct MenuEntry<M: Clone + Debug + 'static> {
        #[widget_core]
        core: kas::CoreData,
        label: Text<AccelString>,
        layout_label: layout::TextStorage,
        layout_frame: layout::FrameStorage,
        msg: M,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.id_ref(), self.label.text().keys());
        }

        fn key_nav(&self) -> bool {
            true
        }
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            let inner = layout::Layout::text(&mut self.layout_label, &mut self.label, TextClass::MenuLabel);
            layout::Layout::frame(&mut self.layout_frame, inner, FrameStyle::MenuEntry)
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let mut draw = draw.with_id(self.id());
            draw.frame(self.core.rect, FrameStyle::MenuEntry);
            draw.text_accel(
                self.layout_label.pos,
                &self.label,
                TextClass::MenuLabel,
            );
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
                label: Text::new_single(label.into()),
                layout_label: Default::default(),
                layout_frame: Default::default(),
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
            if self.label.text().keys() != string.keys() {
                action |= TkAction::RECONFIGURE;
            }
            let avail = self.core.rect.size.clamped_sub(self.layout_frame.size);
            action | kas::text::util::set_text_and_prepare(&mut self.label, string, avail)
        }
    }

    impl Handler for Self {
        type Msg = M;

        fn handle(&mut self, _: &mut EventMgr, event: Event) -> Response<M> {
            match event {
                Event::Activate => self.msg.clone().into(),
                _ => Response::Unused,
            }
        }
    }

    impl Menu for Self {}
}

widget! {
    /// A menu entry which can be toggled
    #[autoimpl(Debug)]
    #[autoimpl(HasBool on self.checkbox)]
    #[derive(Clone, Default)]
    pub struct MenuToggle<M: 'static> {
        #[widget_core]
        core: CoreData,
        #[widget]
        checkbox: CheckBoxBare<M>,
        label: Text<AccelString>,
        layout_label: layout::TextStorage,
        layout_list: layout::DynRowStorage,
        layout_frame: layout::FrameStorage,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.checkbox.id_ref(), self.label.text().keys());
        }
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            let list = [
                layout::Layout::single(&mut self.checkbox),
                layout::Layout::text(&mut self.layout_label, &mut self.label, TextClass::MenuLabel),
            ];
            let inner = layout::Layout::list(list.into_iter(), Direction::Right, &mut self.layout_list);
            layout::Layout::frame(&mut self.layout_frame, inner, FrameStyle::MenuEntry)
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            Some(self.checkbox.id())
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let mut draw = draw.with_id(self.checkbox.id());
            draw.frame(self.core.rect, FrameStyle::MenuEntry);
            self.layout().draw(draw);
        }
    }

    impl Handler for Self where M: From<VoidMsg> {
        type Msg = M;
    }

    impl Menu for Self where M: From<VoidMsg> {}

    impl MenuToggle<VoidMsg> {
        /// Construct a toggleable menu entry with a given `label`
        #[inline]
        pub fn new<T: Into<AccelString>>(label: T) -> Self {
            MenuToggle {
                core: Default::default(),
                checkbox: CheckBoxBare::new(),
                label: Text::new_single(label.into()),
                layout_label: Default::default(),
                layout_list: Default::default(),
                layout_frame: Default::default(),
            }
        }

        /// Set event handler `f`
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The message generated by `f`, if any,
        /// is returned for handling through the parent widget (or other ancestor).
        #[inline]
        #[must_use]
        pub fn on_toggle<M, F>(self, f: F) -> MenuToggle<M>
        where
            F: Fn(&mut EventMgr, bool) -> Option<M> + 'static,
        {
             MenuToggle {
                core: self.core,
                checkbox: self.checkbox.on_toggle(f),
                label: self.label,
                layout_label: self.layout_label,
                layout_list: self.layout_list,
                layout_frame: self.layout_frame,
            }
        }
    }

    impl Self {
        /// Construct a toggleable menu entry with a given `label` and event handler `f`
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The message generated by `f`, if any,
        /// is returned for handling through the parent widget (or other ancestor).
        #[inline]
        pub fn new_on<T: Into<AccelString>, F>(label: T, f: F) -> Self
        where
            F: Fn(&mut EventMgr, bool) -> Option<M> + 'static,
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
