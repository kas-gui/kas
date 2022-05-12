// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Spinner widget

use crate::{EditBox, EditField, EditGuard, MarkButton};
use kas::prelude::*;
use kas::theme::MarkStyle;

#[derive(Clone, Debug)]
enum SpinBtn {
    Down,
    Up,
}

#[derive(Clone, Debug)]
struct SpinnerGuard(i32);
impl EditGuard for SpinnerGuard {
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        if !edit.has_error() {
            mgr.push_msg(edit.guard.0);
        }
    }

    fn focus_lost(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        Self::activate(edit, mgr);
    }

    fn edit(edit: &mut EditField<Self>, _: &mut EventMgr) {
        let parse = edit.get_str().parse();
        edit.set_error_state(parse.is_err());
        if let Ok(val) = parse {
            edit.guard.0 = val;
        }
    }
}

impl_scope! {
    /// A numeric entry widget with up/down arrows
    ///
    /// Sends an `i32` message on edit.
    #[derive(Clone, Debug)]
    #[widget {
        layout = row: [
            self.edit,
            align(center): column: [
                MarkButton::new(MarkStyle::Point(Direction::Up), SpinBtn::Up),
                MarkButton::new(MarkStyle::Point(Direction::Down), SpinBtn::Down),
            ],
        ];
        key_nav = true;
        hover_highlight = true;
    }]
    pub struct Spinner {
        core: widget_core!(),
        #[widget]
        edit: EditBox<SpinnerGuard>,
    }

    impl Self {
        /// Construct
        pub fn new(value: i32) -> Self {
            Spinner {
                core: Default::default(),
                edit: EditBox::new(value.to_string()).with_guard(SpinnerGuard(value)),
            }
        }

        /// Set the initial value
        #[inline]
        #[must_use]
        pub fn with_value(mut self, value: i32) -> Self {
            self.edit.guard.0 = value;
            self
        }

        /// Get the current value
        #[inline]
        pub fn value(&self) -> i32 {
            self.edit.guard.0
        }

        /// Set the value
        ///
        /// Returns [`TkAction::REDRAW`] if a redraw is required.
        pub fn set_value(&mut self, value: i32) -> TkAction {
            if self.edit.guard.0 == value {
                return TkAction::empty();
            }

            self.edit.guard.0 = value;
            self.edit.set_string(value.to_string())
        }
    }

    impl Widget for Self {
        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(btn) = mgr.try_pop_msg() {
                let delta = match btn {
                    SpinBtn::Down => -1,
                    SpinBtn::Up => 1,
                };
                *mgr |= self.set_value(self.value() + delta);
            }
        }
    }
}
