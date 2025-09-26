// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! SpinBox widget

use crate::{EditField, EditGuard, MarkButton};
use kas::messages::{DecrementStep, IncrementStep, ReplaceSelectedText, SetValueF64, SetValueText};
use kas::prelude::*;
use kas::theme::{Background, FrameStyle, MarkStyle, Text, TextClass};
use std::ops::RangeInclusive;

/// Requirements on type used by [`SpinBox`]
///
/// Implementations are provided for standard float and integer types.
///
/// The type must support conversion to and approximate conversion from `f64`
/// in order to enable programmatic control (e.g. tests, accessibility tools).
/// NOTE: this restriction might be revised in the future once Rust supports
/// specialization.
pub trait SpinValue:
    Copy
    + PartialOrd
    + std::fmt::Debug
    + std::str::FromStr
    + ToString
    + Cast<f64>
    + ConvApprox<f64>
    + 'static
{
    /// The default step size (usually 1)
    fn default_step() -> Self;

    /// Add `step` without wrapping
    ///
    /// The implementation should saturate on overflow, at least for fixed-precision types.
    fn add_step(self, step: Self) -> Self;

    /// Subtract `step` without wrapping
    ///
    /// The implementation should saturate on overflow, at least for fixed-precision types.
    fn sub_step(self, step: Self) -> Self;

    /// Clamp `self` to the range `l_bound..=u_bound`
    ///
    /// The default implementation is equivalent to the `std` implementations
    /// for [`Ord`] and for floating-point types.
    fn clamp(self, l_bound: Self, u_bound: Self) -> Self {
        assert!(l_bound <= u_bound);
        if self < l_bound {
            l_bound
        } else if self > u_bound {
            u_bound
        } else {
            self
        }
    }
}

macro_rules! impl_float {
    ($t:ty) => {
        impl SpinValue for $t {
            fn default_step() -> Self {
                1.0
            }
            fn add_step(self, step: Self) -> Self {
                self + step
            }
            fn sub_step(self, step: Self) -> Self {
                self - step
            }
            fn clamp(self, l_bound: Self, u_bound: Self) -> Self {
                <$t>::clamp(self, l_bound, u_bound)
            }
        }
    };
}

impl_float!(f32);
impl_float!(f64);

macro_rules! impl_int {
    ($t:ty) => {
        impl SpinValue for $t {
            fn default_step() -> Self {
                1
            }
            fn add_step(self, step: Self) -> Self {
                self.saturating_add(step)
            }
            fn sub_step(self, step: Self) -> Self {
                self.saturating_sub(step)
            }
            fn clamp(self, l_bound: Self, u_bound: Self) -> Self {
                Ord::clamp(self, l_bound, u_bound)
            }
        }
    };
    ($($t:ty),*) => {
        $(impl_int!($t);)*
    };
}

impl_int!(i8, i16, i32, i64, i128, isize);
impl_int!(u8, u16, u32, u64, u128, usize);

#[derive(Clone, Copy, Debug)]
enum SpinBtn {
    Down,
    Up,
}

#[derive(Debug)]
struct ValueMsg<T>(T);

#[autoimpl(Debug ignore self.state_fn where T: trait)]
struct SpinGuard<A, T: SpinValue> {
    start: T,
    end: T,
    step: T,
    value: T,
    parsed: Option<T>,
    state_fn: Box<dyn Fn(&ConfigCx, &A) -> T>,
}

impl<A, T: SpinValue> SpinGuard<A, T> {
    fn new(range: RangeInclusive<T>, state_fn: Box<dyn Fn(&ConfigCx, &A) -> T>) -> Self {
        let (start, end) = range.into_inner();
        SpinGuard {
            start,
            end,
            step: T::default_step(),
            value: start,
            parsed: None,
            state_fn,
        }
    }

    /// Returns new value if different
    fn handle_btn(&mut self, btn: SpinBtn) -> Option<T> {
        let old_value = self.value;
        let value = match btn {
            SpinBtn::Down => old_value.sub_step(self.step),
            SpinBtn::Up => old_value.add_step(self.step),
        };

        self.value = value.clamp(self.start, self.end);
        (value != old_value).then_some(value)
    }
}

impl<A, T: SpinValue> EditGuard for SpinGuard<A, T> {
    type Data = A;

    fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &A) {
        edit.guard.value = (edit.guard.state_fn)(cx, data);
        edit.set_string(cx, edit.guard.value.to_string());
    }

    fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, _: &A) {
        if let Some(value) = edit.guard.parsed.take() {
            edit.guard.value = value;
            cx.push(ValueMsg(value));
        } else {
            edit.set_string(cx, edit.guard.value.to_string());
        }
    }

    fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &A) {
        let is_err;
        if let Ok(value) = edit.as_str().parse::<T>() {
            edit.guard.value = value.clamp(edit.guard.start, edit.guard.end);
            edit.guard.parsed = Some(edit.guard.value);
            is_err = false;
        } else {
            edit.guard.parsed = None;
            is_err = true;
        };
        edit.set_error_state(cx, is_err);
    }
}

#[impl_self]
mod SpinBox {
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
    ///
    /// ### Messages
    ///
    /// [`SetValueF64`] may be used to set the input value.
    ///
    /// [`IncrementStep`] and [`DecrementStep`] change the value by one step.
    ///
    /// [`SetValueText`] may be used to set the input as a text value.
    /// [`ReplaceSelectedText`] may be used to replace the selected text.
    #[widget]
    #[layout(
        frame!(row![self.edit, self.unit, column! [self.b_up, self.b_down]])
            .with_style(FrameStyle::EditBox)
    )]
    pub struct SpinBox<A, T: SpinValue> {
        core: widget_core!(),
        #[widget]
        edit: EditField<SpinGuard<A, T>>,
        unit: Text<String>,
        #[widget(&())]
        b_up: MarkButton<SpinBtn>,
        #[widget(&())]
        b_down: MarkButton<SpinBtn>,
        on_change: Option<Box<dyn Fn(&mut EventCx, &A, T)>>,
    }

    impl Self {
        /// Construct a spin box
        ///
        /// Values vary within the given `range`. The default step size is
        /// 1 for common types (see [`SpinValue::default_step`]).
        #[inline]
        pub fn new(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
        ) -> Self {
            SpinBox {
                core: Default::default(),
                edit: EditField::new(SpinGuard::new(range, Box::new(state_fn)))
                    .with_width_em(3.0, 8.0),
                unit: Default::default(),
                b_up: MarkButton::new_msg(
                    MarkStyle::Chevron(Direction::Up),
                    "Increment",
                    SpinBtn::Up,
                ),
                b_down: MarkButton::new_msg(
                    MarkStyle::Chevron(Direction::Down),
                    "Decrement",
                    SpinBtn::Down,
                ),
                on_change: None,
            }
        }

        /// Construct a spin box
        ///
        /// - Values vary within the given `range`
        /// - The default step size is 1 for common types (see [`SpinValue::default_step`])
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `msg_fn` is emitted when toggled
        #[inline]
        pub fn new_msg<M: std::fmt::Debug + 'static>(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
            msg_fn: impl Fn(T) -> M + 'static,
        ) -> Self {
            SpinBox::new(range, state_fn).with_msg(msg_fn)
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

        /// Set the unit
        ///
        /// This is an annotation shown after the value.
        pub fn set_unit(&mut self, cx: &mut EventState, unit: impl ToString) {
            self.unit.set_text(unit.to_string());
            let act = self.unit.reprepare_action();
            cx.action(self, act);
        }

        /// Set the unit (inline)
        ///
        /// This method should only be used before the UI has started.
        pub fn with_unit(mut self, unit: impl ToString) -> Self {
            self.unit.set_text(unit.to_string());
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
            kas::MacroDefinedLayout::set_rect(self, cx, rect, hints);
        }

        fn draw(&self, mut draw: DrawCx) {
            let mut draw_edit = draw.re();
            draw_edit.set_id(self.edit.id());
            let bg = if self.edit.has_error() {
                Background::Error
            } else {
                Background::Default
            };
            draw_edit.frame(self.rect(), FrameStyle::EditBox, bg);

            self.edit.draw(draw_edit);
            self.unit.draw(draw.re());
            self.b_up.draw(draw.re());
            self.b_down.draw(draw.re());
        }

        fn probe(&self, coord: Coord) -> Id {
            self.b_up
                .try_probe(coord)
                .or_else(|| self.b_down.try_probe(coord))
                .unwrap_or_else(|| self.edit.id())
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::SpinButton {
                min: self.edit.guard.start.cast(),
                max: self.edit.guard.end.cast(),
                step: self.edit.guard.step.cast(),
                value: self.edit.guard.value.cast(),
            }
        }
    }

    impl Events for Self {
        type Data = A;

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.unit);
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &A, event: Event) -> IsUsed {
            let mut value = None;
            match event {
                Event::Command(cmd, code) => {
                    let btn = match cmd {
                        Command::Down => {
                            cx.depress_with_key(self.b_down.id(), code);
                            SpinBtn::Down
                        }
                        Command::Up => {
                            cx.depress_with_key(self.b_up.id(), code);
                            SpinBtn::Up
                        }
                        _ => return Unused,
                    };
                    value = self.edit.guard.handle_btn(btn);
                }
                Event::Scroll(delta) => {
                    if let Some(y) = delta.as_wheel_action(cx) {
                        let (count, btn) = if y > 0 {
                            (y as u32, SpinBtn::Up)
                        } else {
                            ((-y) as u32, SpinBtn::Down)
                        };
                        for _ in 0..count {
                            value = self.edit.guard.handle_btn(btn);
                        }
                    } else {
                        return Unused;
                    }
                }
                _ => return Unused,
            }

            if let Some(value) = value {
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
                self.edit.guard.handle_btn(btn)
            } else if let Some(SetValueF64(v)) = cx.try_pop() {
                match v.try_cast_approx() {
                    Ok(value) => Some(value),
                    Err(err) => {
                        log::warn!("Slider failed to handle SetValueF64: {err}");
                        None
                    }
                }
            } else if let Some(IncrementStep) = cx.try_pop() {
                Some(self.edit.guard.value.add_step(self.edit.guard.step))
            } else if let Some(DecrementStep) = cx.try_pop() {
                Some(self.edit.guard.value.sub_step(self.edit.guard.step))
            } else if let Some(SetValueText(string)) = cx.try_pop() {
                self.edit.set_string(cx, string);
                SpinGuard::edit(&mut self.edit, cx, data);
                self.edit.guard.parsed
            } else if let Some(ReplaceSelectedText(text)) = cx.try_pop() {
                self.edit.replace_selection(cx, &text);
                SpinGuard::edit(&mut self.edit, cx, data);
                self.edit.guard.parsed
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
