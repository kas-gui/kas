// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Spinner widget

use crate::{EditField, EditGuard, MarkButton};
use kas::event::{Command, ScrollDelta};
use kas::prelude::*;
use kas::theme::{Background, FrameStyle, MarkStyle, TextClass};
use std::cmp::Ord;
use std::ops::RangeInclusive;
use std::rc::Rc;

/// Requirements on type used by [`Spinner`]
///
/// Implementations are provided for standard float and integer types.
pub trait SpinnerValue:
    Copy + PartialOrd + std::fmt::Debug + std::str::FromStr + ToString + 'static
{
    /// Clamp `self` to the range `l_bound..=u_bound`
    fn clamp(self, l_bound: Self, u_bound: Self) -> Self;

    /// Add `x` without overflow, clamping the result to no more than `u_bound`
    fn add_step(self, step: Self, u_bound: Self) -> Self;

    /// Subtract `step` without overflow, clamping the result to no less than `l_bound`
    fn sub_step(self, step: Self, l_bound: Self) -> Self;
}

macro_rules! impl_float {
    ($t:ty) => {
        impl SpinnerValue for $t {
            fn clamp(self, l_bound: Self, u_bound: Self) -> Self {
                <$t>::clamp(self, l_bound, u_bound)
            }

            fn add_step(self, step: Self, u_bound: Self) -> Self {
                ((self / step + 1.0).round() * step).min(u_bound)
            }

            fn sub_step(self, step: Self, l_bound: Self) -> Self {
                ((self / step - 1.0).round() * step).max(l_bound)
            }
        }
    };
}

impl_float!(f32);
impl_float!(f64);

macro_rules! impl_int {
    ($t:ty) => {
        impl SpinnerValue for $t {
            fn clamp(self, l_bound: Self, u_bound: Self) -> Self {
                Ord::clamp(self, l_bound, u_bound)
            }

            fn add_step(self, step: Self, u_bound: Self) -> Self {
                ((self / step).saturating_add(1)).saturating_mul(step).min(u_bound)
            }

            fn sub_step(self, step: Self, l_bound: Self) -> Self {
                (((self + step - 1) / step).saturating_sub(1)).saturating_mul(step).max(l_bound)
            }
        }
    };
    ($($t:ty),*) => {
        $(impl_int!($t);)*
    };
}

impl_int!(i8, i16, i32, i64, i128, isize);
impl_int!(u8, u16, u32, u64, u128, usize);

#[derive(Clone, Debug)]
enum SpinBtn {
    Down,
    Up,
}

#[derive(Debug)]
struct ValueMsg<T>(T);

#[derive(Clone, Debug)]
struct SpinnerGuard<T: SpinnerValue> {
    value: T,
    start: T,
    end: T,
}

impl<T: SpinnerValue> SpinnerGuard<T> {
    fn new(range: RangeInclusive<T>) -> Self {
        let (start, end) = range.into_inner();
        SpinnerGuard {
            value: start,
            start,
            end,
        }
    }

    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn set_value(&mut self, value: T) {
        self.value = value.clamp(self.start, self.end);
    }

    fn range(&self) -> RangeInclusive<T> {
        self.start..=self.end
    }
}

impl<T: SpinnerValue> EditGuard for SpinnerGuard<T> {
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        if edit.has_error() {
            *mgr |= edit.set_string(edit.guard.value.to_string());
            edit.set_error_state(false);
        }
        mgr.push_msg(ValueMsg(edit.guard.value));
    }

    fn focus_lost(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        if edit.has_error() {
            *mgr |= edit.set_string(edit.guard.value.to_string());
            edit.set_error_state(false);
        }
    }

    fn edit(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        let is_err = match edit.get_str().parse() {
            Ok(value) if edit.guard.range().contains(&value) => {
                if value != edit.guard.value {
                    edit.guard.value = value;
                    mgr.push_msg(ValueMsg(value));
                }
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
    /// The value is constrained to a given `range`. Increment and decrement
    /// operations advance to the next/previous multiple of `step`.
    ///
    /// Recommendations for optimal behaviour:
    ///
    /// -   Ensure that range end points are a multiple of `step`
    /// -   With floating-point types, ensure that `step` is exactly
    ///     representable, e.g. an integer or a power of 2.
    #[autoimpl(Debug ignore self.on_change)]
    #[widget {
        layout = frame(FrameStyle::EditBox): row: [
            self.edit,
            column: [
                margins(none): self.b_up,
                margins(none): self.b_down,
            ],
        ];
    }]
    pub struct Spinner<T: SpinnerValue> {
        core: widget_core!(),
        #[widget]
        edit: EditField<SpinnerGuard<T>>,
        #[widget]
        b_up: MarkButton<SpinBtn>,
        #[widget]
        b_down: MarkButton<SpinBtn>,
        step: T,
        on_change: Option<Rc<dyn Fn(&mut EventMgr, T)>>,
    }

    impl Self {
        /// Construct a spinner with given `range` and `step`
        #[inline]
        pub fn new(range: RangeInclusive<T>, step: T) -> Self {
            Spinner {
                core: Default::default(),
                edit: EditField::new("")
                    .with_width_em(3.0, 8.0)
                    .with_guard(SpinnerGuard::new(range)),
                b_up: MarkButton::new(MarkStyle::Point(Direction::Up), SpinBtn::Up),
                b_down: MarkButton::new(MarkStyle::Point(Direction::Down), SpinBtn::Down),
                step,
                on_change: None,
            }
        }

        /// Construct a spinner with event handler `f`
        ///
        /// This closure is called when the value is changed.
        #[inline]
        pub fn new_on<F>(range: RangeInclusive<T>, step: T, f: F) -> Self
        where
            F: Fn(&mut EventMgr, T) + 'static,
        {
            Spinner::new(range, step).on_change(f)
        }

        /// Set event handler `f`
        ///
        /// This closure is called when the value is changed, specifically:
        ///
        /// -   If the increment/decrement buttons, <kbd>Up</kbd>/<kbd>Down</kbd>
        ///     keys or mouse scroll wheel is used and the value changes
        /// -   If the value is adjusted via the edit box and the result is valid
        /// -   If <kbd>Enter</kbd> is pressed in the edit box
        #[inline]
        #[must_use]
        pub fn on_change<F>(mut self, f: F) -> Self
        where
            F: Fn(&mut EventMgr, T) + 'static,
        {
            self.on_change = Some(Rc::new(f));
            self
        }

        /// Set the text class used
        ///
        /// The default is: `TextClass::Edit(false)`.
        #[inline]
        #[must_use]
        pub fn with_class(mut self, class: TextClass) -> Self {
            self.edit = self.edit.with_class(class);
            self
        }

        /// Get the text class used
        #[inline]
        pub fn class(&self) -> TextClass {
            self.edit.class()
        }

        /// Adjust the width allocation
        #[inline]
        pub fn set_width_em(&mut self, min_em: f32, ideal_em: f32) {
            self.edit.set_width_em(min_em, ideal_em);
        }

        /// Adjust the width allocation (inline)
        #[inline]
        #[must_use]
        pub fn with_width_em(mut self, min_em: f32, ideal_em: f32) -> Self {
            self.set_width_em(min_em, ideal_em);
            self
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
            self.edit.guard.value
        }

        /// Set the value
        ///
        /// Returns [`TkAction::REDRAW`] if a redraw is required.
        pub fn set_value(&mut self, value: T) -> TkAction {
            self.edit.set_error_state(false);
            let old_value = self.edit.guard.value;
            self.edit.guard.set_value(value);
            if self.edit.guard.value != old_value {
                self.edit.set_string(self.edit.guard.value.to_string())
            } else {
                TkAction::empty()
            }
        }

        fn handle_btn(&mut self, mgr: &mut EventMgr, btn: SpinBtn) {
            let value = match btn {
                SpinBtn::Down => self.value().sub_step(self.step, self.edit.guard.start),
                SpinBtn::Up => self.value().add_step(self.step, self.edit.guard.end),
            };

            if value != self.edit.guard.value {
                self.edit.guard.value = value;
                if let Some(ref f) = self.on_change {
                    f(mgr, value);
                }
            }

            self.edit.set_error_state(false);
            *mgr |= self.edit.set_string(value.to_string());
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
        fn configure(&mut self, mgr: &mut ConfigMgr) {
            *mgr |= self.edit.set_string(self.edit.guard.value.to_string());
        }

        fn steal_event(&mut self, mgr: &mut EventMgr, _: &WidgetId, event: &Event) -> Response {
            let btn = match event {
                Event::Command(cmd) => {
                    match cmd {
                        Command::Down => SpinBtn::Down,
                        Command::Up => SpinBtn::Up,
                        _ => return Response::Unused,
                    }
                }
                Event::Scroll(ScrollDelta::LineDelta(_, y)) => {
                    if *y > 0.0 {
                        SpinBtn::Up
                    } else if *y < 0.0 {
                        SpinBtn::Down
                    } else {
                        return Response::Unused;
                    }
                }
                _ => return Response::Unused,
            };

            self.handle_btn(mgr, btn);
            Response::Used
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(ValueMsg(value)) = mgr.try_pop_msg() {
                if let Some(ref f) = self.on_change {
                    f(mgr, value);
                }
            }
            if let Some(btn) = mgr.try_pop_msg::<SpinBtn>() {
                self.handle_btn(mgr, btn);
            }
        }
    }
}
