// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Spinner widget

use crate::{EditField, EditGuard, MarkButton};
use kas::event::{Command, ScrollDelta};
use kas::prelude::*;
use kas::theme::{Background, FrameStyle, MarkStyle};
use std::ops::{Add, RangeInclusive, Sub};

/// Requirements on type used by [`Spinner`]
pub trait SpinnerType:
    Copy
    + Add<Output = Self>
    + Sub<Output = Self>
    + PartialOrd
    + std::fmt::Debug
    + std::str::FromStr
    + ToString
    + Sized
    + 'static
{
}
impl<
        T: Copy
            + Add<Output = Self>
            + Sub<Output = Self>
            + PartialOrd
            + std::fmt::Debug
            + std::str::FromStr
            + ToString
            + Sized
            + 'static,
    > SpinnerType for T
{
}

#[derive(Clone, Debug)]
enum SpinBtn {
    Down,
    Up,
}

#[derive(Clone, Debug)]
struct SpinnerGuard<T: SpinnerType>(T, RangeInclusive<T>);
impl<T: SpinnerType> SpinnerGuard<T> {
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn set_value(&mut self, value: T) {
        self.0 = if !(value >= *self.1.start()) {
            *self.1.start()
        } else if !(value <= *self.1.end()) {
            *self.1.end()
        } else {
            value
        };
    }
}
impl<T: SpinnerType> EditGuard for SpinnerGuard<T> {
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        if edit.has_error() {
            *mgr |= edit.set_string(edit.guard.0.to_string());
            edit.set_error_state(false);
        }
        mgr.push_msg(edit.guard.0);
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
    /// Sends a message of type `T` on edit.
    #[derive(Clone, Debug)]
    #[widget {
        layout = frame(FrameStyle::EditBox): row: [
            self.edit,
            align(stretch): column: [self.b_up, self.b_down],
        ];
    }]
    pub struct Spinner<T: SpinnerType> {
        core: widget_core!(),
        #[widget]
        edit: EditField<SpinnerGuard<T>>,
        #[widget]
        b_up: MarkButton<SpinBtn>,
        #[widget]
        b_down: MarkButton<SpinBtn>,
        step: T,
    }

    impl Self {
        /// Construct
        pub fn new(range: RangeInclusive<T>, step: T) -> Self {
            assert!(!range.is_empty());
            let min = *range.start();
            let mut guard = SpinnerGuard(min, range);
            guard.set_value(min);

            Spinner {
                core: Default::default(),
                edit: EditField::new(guard.0.to_string()).with_guard(guard),
                b_up: MarkButton::new(MarkStyle::Point(Direction::Up), SpinBtn::Up),
                b_down: MarkButton::new(MarkStyle::Point(Direction::Down), SpinBtn::Down),
                step,
            }
        }

        /// Set the initial value
        #[inline]
        #[must_use]
        pub fn with_value(mut self, value: T) -> Self {
            self.edit.guard.set_value(value);
            self
        }

        /// Get the current value
        #[inline]
        pub fn value(&self) -> T {
            self.edit.guard.0
        }

        /// Set the value
        ///
        /// Returns [`TkAction::REDRAW`] if a redraw is required.
        pub fn set_value(&mut self, value: T) -> TkAction {
            if self.edit.guard.0 == value {
                return TkAction::empty();
            }

            self.edit.guard.set_value(value);
            self.edit.set_error_state(false);
            self.edit.set_string(self.edit.guard.0.to_string())
        }

        fn set_and_emit(&mut self, mgr: &mut EventMgr, value: T) -> Response {
            *mgr |= self.set_value(value);
            mgr.push_msg(self.value());
            Response::Used
        }
    }

    impl Layout for Self {
        fn draw(&mut self, mut draw: DrawMgr) {
            let bg = if self.edit.has_error() {
                Background::Error
            } else {
                Background::Default
            };
            draw.frame(self.rect(), FrameStyle::EditBox, bg);
            draw.recurse(&mut self.edit);
            draw.recurse(&mut self.b_up);
            draw.recurse(&mut self.b_down);
        }
    }

    impl Widget for Self {
        fn steal_event(&mut self, mgr: &mut EventMgr, _: &WidgetId, event: &Event) -> Response {
            match event {
                Event::Command(cmd, _) => {
                    let value = match cmd {
                        Command::Down => self.value() - self.step,
                        Command::Up => self.value() + self.step,
                        _ => return Response::Unused,
                    };
                    self.set_and_emit(mgr, value)
                }
                Event::Scroll(ScrollDelta::LineDelta(_, y)) => {
                    if *y > 0.0 {
                        self.set_and_emit(mgr, self.value() + self.step)
                    } else if *y < 0.0 {
                        self.set_and_emit(mgr, self.value() - self.step)
                    } else {
                        Response::Unused
                    }
                }
                _ => Response::Unused,
            }
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(btn) = mgr.try_pop_msg() {
                let value = match btn {
                    SpinBtn::Down => self.value() - self.step,
                    SpinBtn::Up => self.value() + self.step,
                };
                *mgr |= self.set_value(value);
                mgr.push_msg(self.value());
            }
        }
    }
}
