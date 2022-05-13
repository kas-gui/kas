// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Spinner widget

use crate::{EditBox, EditField, EditGuard, MarkButton};
use kas::prelude::*;
use kas::theme::MarkStyle;
use std::ops::RangeInclusive;

#[derive(Clone, Debug)]
enum SpinBtn {
    Down,
    Up,
}

#[derive(Clone, Debug)]
struct SpinnerGuard(i32, RangeInclusive<i32>);
impl SpinnerGuard {
    fn set_value(&mut self, value: i32) {
        self.0 = value.min(*self.1.end()).max(*self.1.start());
    }
}
impl EditGuard for SpinnerGuard {
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        if edit.has_error() {
            *mgr |= edit.set_string(edit.guard.0.to_string());
            edit.set_error_state(false);
        } else {
            mgr.push_msg(edit.guard.0);
        }
    }

    fn focus_lost(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        Self::activate(edit, mgr);
    }

    fn edit(edit: &mut EditField<Self>, _: &mut EventMgr) {
        let is_err = match edit.get_str().parse() {
            Ok(value) if edit.guard.1.contains(&value) => {
                edit.guard.0 = value;
                false
            }
            Ok(value) => {
                edit.guard.set_value(value);
                true
            }
            _ => true,
        };
        edit.set_error_state(is_err);
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
        pub fn new(range: RangeInclusive<i32>) -> Self {
            let mut guard = SpinnerGuard(0, range);
            guard.set_value(0);

            Spinner {
                core: Default::default(),
                edit: EditBox::new(guard.0.to_string()).with_guard(guard),
            }
        }

        /// Set the initial value
        #[inline]
        #[must_use]
        pub fn with_value(mut self, value: i32) -> Self {
            self.edit.guard.set_value(value);
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

            self.edit.guard.set_value(value);
            self.edit.set_error_state(false);
            self.edit.set_string(self.edit.guard.0.to_string())
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
