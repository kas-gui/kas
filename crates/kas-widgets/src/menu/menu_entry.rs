// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu Entries

use super::Menu;
use crate::{AccelLabel, CheckBoxBare};
use kas::draw::TextClass;
use kas::prelude::*;
use std::fmt::Debug;

widget! {
    /// A standard menu entry
    #[derive(Clone, Debug, Default)]
    pub struct MenuEntry<M: Clone + Debug + 'static> {
        #[widget_core]
        core: kas::CoreData,
        label: Text<AccelString>,
        label_off: Offset,
        frame_size: Size,
        msg: M,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut Manager) {
            mgr.add_accel_keys(self.id(), self.label.text().keys());
        }

        fn key_nav(&self) -> bool {
            true
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
            let frame_rules = size_handle.menu_frame(axis.is_vertical());
            let text_rules = size_handle.text_bound(&mut self.label, TextClass::MenuLabel, axis);
            let (rules, offset, size) = frame_rules.surround_as_margin(text_rules);
            self.label_off.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            let size = rect.size - self.frame_size;
            self.label.update_env(|env| {
                env.set_bounds(size.into());
                env.set_align(align.unwrap_or(Align::Default, Align::Centre));
            });
        }

        fn draw(&self, draw: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            draw.menu_entry(self.core.rect, self.input_state(mgr, disabled));
            let pos = self.core.rect.pos + self.label_off;
            draw.text_accel(
                pos,
                &self.label,
                mgr.show_accel_labels(),
                TextClass::MenuLabel,
                self.input_state(mgr, disabled),
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
                label_off: Offset::ZERO,
                frame_size: Size::ZERO,
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
            let avail = self.core.rect.size.clamped_sub(self.frame_size);
            action | kas::text::util::set_text_and_prepare(&mut self.label, string, avail)
        }
    }

    impl Handler for Self {
        type Msg = M;

        fn handle(&mut self, _: &mut Manager, event: Event) -> Response<M> {
            match event {
                Event::Activate => self.msg.clone().into(),
                _ => Response::Unhandled,
            }
        }
    }

    impl Menu for Self {}
}

widget! {
    /// A menu entry which can be toggled
    #[autoimpl(Debug)]
    #[autoimpl(HasBool on checkbox)]
    #[derive(Clone, Default)]
    #[layout(row, area=checkbox, draw=draw)]
    pub struct MenuToggle<M: 'static> {
        #[widget_core]
        core: CoreData,
        #[widget]
        checkbox: CheckBoxBare<M>,
        // TODO: label should use TextClass::MenuLabel
        #[widget]
        label: AccelLabel,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut Manager) {
            mgr.add_accel_keys(self.checkbox.id(), self.label.keys());
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
                label: AccelLabel::new(label.into()),
            }
        }

        /// Set event handler `f`
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The message generated by `f`, if any,
        /// is returned for handling through the parent widget (or other ancestor).
        #[inline]
        pub fn on_toggle<M, F>(self, f: F) -> MenuToggle<M>
        where
            F: Fn(&mut Manager, bool) -> Option<M> + 'static,
        {
            MenuToggle {
                core: self.core,
                checkbox: self.checkbox.on_toggle(f),
                label: self.label,
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
            F: Fn(&mut Manager, bool) -> Option<M> + 'static,
        {
            MenuToggle::new(label).on_toggle(f)
        }

        /// Set the initial state of the checkbox.
        #[inline]
        pub fn with_state(mut self, state: bool) -> Self {
            self.checkbox = self.checkbox.with_state(state);
            self
        }

        fn draw(&self, draw: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            let state = self.checkbox.input_state(mgr, disabled);
            draw.menu_entry(self.core.rect, state);
            self.checkbox.draw(draw, mgr, state.disabled());
            self.label.draw(draw, mgr, state.disabled());
        }
    }
}
