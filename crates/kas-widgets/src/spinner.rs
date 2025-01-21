// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Spinner widget

use crate::{EditField, EditGuard, MarkButton};
use kas::event::{Command, ScrollDelta};
use kas::prelude::*;
use kas::theme::{FrameStyle, MarkStyle, TextClass};
use std::ops::RangeInclusive;

/// Requirements on type used by [`Spinner`]
///
/// Implementations are provided for standard float and integer types.
pub trait SpinnerValue:
    Copy + PartialOrd + std::fmt::Debug + std::str::FromStr + ToString + 'static
{
    /// The default step size (usually 1)
    fn default_step() -> Self;

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
            fn default_step() -> Self {
                1.0
            }
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
            fn default_step() -> Self {
                1
            }
            fn clamp(self, l_bound: Self, u_bound: Self) -> Self {
                Ord::clamp(self, l_bound, u_bound)
            }

            fn add_step(self, step: Self, u_bound: Self) -> Self {
                ((self / step).saturating_add(1)).saturating_mul(step).min(u_bound)
            }
            fn sub_step(self, step: Self, l_bound: Self) -> Self {
                #[allow(clippy::manual_div_ceil)] // only stable on a subset of types used
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

#[autoimpl(Debug ignore self.state_fn where T: trait)]
struct SpinnerGuard<A, T: SpinnerValue> {
    start: T,
    end: T,
    step: T,
    parsed: Option<T>,
    state_fn: Box<dyn Fn(&ConfigCx, &A) -> T>,
}

impl<A, T: SpinnerValue> SpinnerGuard<A, T> {
    fn new(range: RangeInclusive<T>, state_fn: Box<dyn Fn(&ConfigCx, &A) -> T>) -> Self {
        let (start, end) = range.into_inner();
        SpinnerGuard {
            start,
            end,
            step: T::default_step(),
            parsed: None,
            state_fn,
        }
    }

    /// Returns new value if different
    fn handle_btn(&self, cx: &mut EventCx, data: &A, btn: SpinBtn) -> Option<T> {
        let old_value = (self.state_fn)(&cx.config_cx(), data);
        let value = match btn {
            SpinBtn::Down => old_value.sub_step(self.step, self.start),
            SpinBtn::Up => old_value.add_step(self.step, self.end),
        };

        (value != old_value).then_some(value)
    }
}

impl<A, T: SpinnerValue> EditGuard for SpinnerGuard<A, T> {
    type Data = A;

    fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &A) {
        let value = (edit.guard.state_fn)(cx, data);
        let action = edit.set_string(value.to_string());
        cx.action(edit, action);
    }

    fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, data: &A) {
        if let Some(value) = edit.guard.parsed.take() {
            cx.push(ValueMsg(value));
        } else {
            let value = (edit.guard.state_fn)(&cx.config_cx(), data);
            let action = edit.set_string(value.to_string());
            cx.action(edit, action);
        }
    }

    fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &A) {
        let is_err;
        if let Ok(value) = edit.get_str().parse::<T>() {
            edit.guard.parsed = Some(value.clamp(edit.guard.start, edit.guard.end));
            is_err = false;
        } else {
            is_err = true;
        };
        let action = edit.set_error_state(is_err);
        cx.action(edit, action);
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
    #[widget {
        layout = frame!(row! [
            self.edit,
            column! [self.b_up, self.b_down],
        ], style = FrameStyle::EditBox);
    }]
    pub struct Spinner<A, T: SpinnerValue> {
        core: widget_core!(),
        #[widget]
        edit: EditField<SpinnerGuard<A, T>>,
        #[widget(&())]
        b_up: MarkButton<SpinBtn>,
        #[widget(&())]
        b_down: MarkButton<SpinBtn>,
        on_change: Option<Box<dyn Fn(&mut EventCx, &A, T)>>,
    }

    impl Self {
        /// Construct a spinner
        ///
        /// Values vary within the given `range`. The default step size is
        /// 1 for common types (see [`SpinnerValue::default_step`]).
        #[inline]
        pub fn new(range: RangeInclusive<T>, state_fn: impl Fn(&ConfigCx, &A) -> T + 'static) -> Self {
            Spinner {
                core: Default::default(),
                edit: EditField::new(SpinnerGuard::new(range, Box::new(state_fn)))
                    .with_width_em(3.0, 8.0),
                b_up: MarkButton::new_msg(MarkStyle::Point(Direction::Up), SpinBtn::Up),
                b_down: MarkButton::new_msg(MarkStyle::Point(Direction::Down), SpinBtn::Down),
                on_change: None,
            }
        }

        /// Construct a spinner
        ///
        /// - Values vary within the given `range`
        /// - The default step size is 1 for common types (see [`SpinnerValue::default_step`])
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `msg_fn` is emitted when toggled
        #[inline]
        pub fn new_msg<M: std::fmt::Debug + 'static>(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
            msg_fn: impl Fn(T) -> M + 'static,
        ) -> Self {
            Spinner::new(range, state_fn).with_msg(msg_fn)
        }

        /// Send the message generated by `f` on change
        #[inline]
        #[must_use]
        pub fn with_msg<M>(self, f: impl Fn(T) -> M + 'static) -> Self
        where
            M: std::fmt::Debug + 'static,
        {
            self.with(move |cx, _, state| cx.push(f(state)))
        }

        /// Call the handler `f` on change
        ///
        /// This closure is called when the value is changed, specifically:
        ///
        /// -   If the increment/decrement buttons, <kbd>Up</kbd>/<kbd>Down</kbd>
        ///     keys or mouse scroll wheel is used and the value changes
        /// -   If the value is adjusted via the edit box and the result is valid
        /// -   If <kbd>Enter</kbd> is pressed in the edit box
        #[inline]
        #[must_use]
        pub fn with(mut self, f: impl Fn(&mut EventCx, &A, T) + 'static) -> Self {
            debug_assert!(self.on_change.is_none());
            self.on_change = Some(Box::new(f));
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

        /// Set the step size
        #[inline]
        #[must_use]
        pub fn with_step(mut self, step: T) -> Self {
            self.edit.guard.step = step;
            self
        }
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.core.rect = rect;
            self.layout_visitor().set_rect(cx, rect, hints);
            self.edit.set_outer_rect(rect, FrameStyle::EditBox);
        }

        fn draw(&mut self, mut draw: DrawCx) {
            self.edit.draw(draw.re());
            self.b_up.draw(draw.re());
            self.b_down.draw(draw.re());
        }
    }

    impl Events for Self {
        type Data = A;

        fn probe(&mut self, coord: Coord) -> Id {
            self.b_up.try_probe(coord)
                .or_else(|| self.b_down.try_probe(coord))
                .unwrap_or_else(|| self.edit.id())
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &A, event: Event) -> IsUsed {
            let btn = match event {
                Event::Command(cmd, code) => match cmd {
                    Command::Down => {
                        cx.depress_with_key(self.b_down.id(), code);
                        SpinBtn::Down
                    }
                    Command::Up => {
                        cx.depress_with_key(self.b_up.id(), code);
                        SpinBtn::Up
                    }
                    _ => return Unused,
                },
                Event::Scroll(ScrollDelta::LineDelta(_, y)) => {
                    if y > 0.0 {
                        SpinBtn::Up
                    } else if y < 0.0 {
                        SpinBtn::Down
                    } else {
                        return Unused;
                    }
                }
                _ => return Unused,
            };

            if let Some(value) = self.edit.guard.handle_btn(cx, data, btn) {
                if let Some(ref f) = self.on_change {
                    f(cx, data, value);
                }
            }
            Used
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &A) {
            let new_value = if let Some(ValueMsg(value)) = cx.try_pop() {
                Some(value)
            } else if let Some(btn) = cx.try_pop::<SpinBtn>() {
                self.edit.guard.handle_btn(cx, data, btn)
            } else {
                None
            };

            if let Some(value) = new_value {
                if let Some(ref f) = self.on_change {
                    f(cx, data, value);
                }
            }
        }
    }
}
