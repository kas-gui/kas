// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Slider` control

use super::{GripMsg, GripPart};
use kas::event::{Command, FocusSource};
use kas::messages::{DecrementStep, IncrementStep, SetValueF64};
use kas::prelude::*;
use kas::theme::Feature;
use std::fmt::Debug;
use std::ops::{Add, RangeInclusive, Sub};

/// Requirements on type used by [`Slider`]
///
/// Implementations are provided for standard float and integer types.
pub trait SliderValue:
    Copy
    + Debug
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Cast<f64>
    + ConvApprox<f64>
    + 'static
{
    /// The default step size (usually 1)
    fn default_step() -> Self;
}

impl SliderValue for f64 {
    fn default_step() -> Self {
        1.0
    }
}

impl SliderValue for f32 {
    fn default_step() -> Self {
        1.0
    }
}

macro_rules! impl_slider_ty {
    ($ty:ty) => {
        impl SliderValue for $ty {
            fn default_step() -> Self {
                1
            }
        }
    };
    ($ty:ty, $($tt:ty),*) => {
        impl_slider_ty!($ty);
        impl_slider_ty!($($tt),*);
    };
}
impl_slider_ty!(i8, i16, i32, i64, i128, isize);
impl_slider_ty!(u8, u16, u32, u64, u128, usize);

#[impl_self]
mod Slider {
    /// A slider
    ///
    /// Sliders allow user input of a value from a fixed range.
    ///
    /// ### Messages
    ///
    /// [`SetValueF64`] may be used to set the input value.
    ///
    /// [`IncrementStep`] and [`DecrementStep`] change the value by one step.
    #[autoimpl(Debug ignore self.state_fn, self.on_move)]
    #[widget]
    pub struct Slider<A, T: SliderValue, D: Directional = Direction> {
        core: widget_core!(),
        direction: D,
        // Terminology assumes vertical orientation:
        range: (T, T),
        step: T,
        value: T,
        #[widget(&())]
        grip: GripPart,
        state_fn: Box<dyn Fn(&ConfigCx, &A) -> T>,
        on_move: Option<Box<dyn Fn(&mut EventCx, &A, T)>>,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a slider
        ///
        /// Values vary within the given `range`, increasing in the given
        /// `direction`. The default step size is
        /// 1 for common types (see [`SliderValue::default_step`]).
        ///
        /// The slider's current value is set by `state_fn` on update.
        ///
        /// To make the slider interactive, assign an event handler with
        /// [`Self::with`] or [`Self::with_msg`].
        #[inline]
        pub fn new(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
        ) -> Self {
            Slider::new_dir(range, state_fn, D::default())
        }
    }
    impl<A, T: SliderValue> Slider<A, T, kas::dir::Left> {
        /// Construct with fixed direction
        #[inline]
        pub fn left(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
        ) -> Self {
            Slider::new(range, state_fn)
        }
    }
    impl<A, T: SliderValue> Slider<A, T, kas::dir::Right> {
        /// Construct with fixed direction
        #[inline]
        pub fn right(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
        ) -> Self {
            Slider::new(range, state_fn)
        }
    }
    impl<A, T: SliderValue> Slider<A, T, kas::dir::Up> {
        /// Construct with fixed direction
        #[inline]
        pub fn up(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
        ) -> Self {
            Slider::new(range, state_fn)
        }
    }

    impl<A, T: SliderValue> Slider<A, T, kas::dir::Down> {
        /// Construct with fixed direction
        #[inline]
        pub fn down(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
        ) -> Self {
            Slider::new(range, state_fn)
        }
    }

    impl Self {
        /// Construct a slider with given direction
        ///
        /// Values vary within the given `range`, increasing in the given
        /// `direction`. The default step size is
        /// 1 for common types (see [`SliderValue::default_step`]).
        ///
        /// The slider's current value is set by `state_fn` on update.
        ///
        /// To make the slider interactive, assign an event handler with
        /// [`Self::with`] or [`Self::with_msg`].
        #[inline]
        pub fn new_dir(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
            direction: D,
        ) -> Self {
            assert!(!range.is_empty());
            let value = *range.start();
            Slider {
                core: Default::default(),
                direction,
                range: range.into_inner(),
                step: T::default_step(),
                value,
                grip: GripPart::new(),
                state_fn: Box::new(state_fn),
                on_move: None,
            }
        }

        /// Send the message generated by `f` on movement
        #[inline]
        #[must_use]
        pub fn with_msg<M>(self, f: impl Fn(T) -> M + 'static) -> Self
        where
            M: std::fmt::Debug + 'static,
        {
            self.with(move |cx, _, state| cx.push(f(state)))
        }

        /// Call the handler `f` on movement
        #[inline]
        #[must_use]
        pub fn with(mut self, f: impl Fn(&mut EventCx, &A, T) + 'static) -> Self {
            debug_assert!(self.on_move.is_none());
            self.on_move = Some(Box::new(f));
            self
        }

        /// Get the slider's direction
        #[inline]
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Set the step size
        #[inline]
        #[must_use]
        pub fn with_step(mut self, step: T) -> Self {
            self.step = step;
            self
        }

        /// Set value and update grip
        ///
        /// Returns `true` if, after clamping to the supported range, `value`
        /// differs from the existing value.
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        fn set_value(&mut self, cx: &mut EventState, value: T) -> bool {
            let value = if !(value >= self.range.0) {
                self.range.0
            } else if !(value <= self.range.1) {
                self.range.1
            } else {
                value
            };

            if value == self.value {
                return false;
            }

            self.value = value;
            self.grip.set_offset(cx, self.offset());
            true
        }

        // translate value to offset in local coordinates
        fn offset(&self) -> Offset {
            let a = self.value - self.range.0;
            let b = self.range.1 - self.range.0;
            let max_offset = self.grip.max_offset();
            let mut frac = a.cast() / b.cast();
            assert!((0.0..=1.0).contains(&frac));
            if self.direction.is_reversed() {
                frac = 1.0 - frac;
            }
            match self.direction.is_vertical() {
                false => Offset((max_offset.0 as f64 * frac).cast_floor(), 0),
                true => Offset(0, (max_offset.1 as f64 * frac).cast_floor()),
            }
        }

        fn apply_grip_offset(&mut self, cx: &mut EventCx, data: &A, offset: Offset) {
            let b = self.range.1 - self.range.0;
            let max_offset = self.grip.max_offset();
            let (offset, max) = match self.direction.is_vertical() {
                false => (offset.0, max_offset.0),
                true => (offset.1, max_offset.1),
            };
            let mut a = (b.cast() * (offset as f64 / max as f64))
                .round()
                .cast_approx();
            if self.direction.is_reversed() {
                a = b - a;
            }
            if self.set_value(cx, a + self.range.0) {
                if let Some(ref f) = self.on_move {
                    f(cx, data, self.value);
                }
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let _ = self.grip.size_rules(sizer.re(), axis);
            sizer.feature(Feature::Slider(self.direction()), axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            let align = match self.direction.is_vertical() {
                false => AlignPair::new(Align::Stretch, hints.vert.unwrap_or(Align::Center)),
                true => AlignPair::new(hints.horiz.unwrap_or(Align::Center), Align::Stretch),
            };
            let mut rect = cx.align_feature(Feature::Slider(self.direction()), rect, align);
            widget_set_rect!(rect);
            self.grip.set_track(rect);

            // Set the grip size (we could instead call set_size but the widget
            // model requires we call set_rect anyway):
            rect.size
                .set_component(self.direction, cx.size_cx().grip_len());
            self.grip.set_rect(cx, rect, AlignHints::NONE);
            // Correct the position:
            self.grip.set_offset(cx, self.offset());
        }

        fn draw(&self, mut draw: DrawCx) {
            let dir = self.direction.as_direction();
            draw.slider(self.rect(), &self.grip, dir);
        }
    }

    impl Tile for Self {
        fn navigable(&self) -> bool {
            true
        }

        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Slider {
                min: self.range.0.cast(),
                max: self.range.1.cast(),
                step: self.step.cast(),
                value: self.value.cast(),
                direction: self.direction.as_direction(),
            }
        }

        fn probe(&self, coord: Coord) -> Id {
            if self.on_move.is_some() {
                if let Some(id) = self.grip.try_probe(coord) {
                    return id;
                }
            }
            self.id()
        }
    }

    impl Events for Self {
        const REDRAW_ON_MOUSE_OVER: bool = true;

        type Data = A;

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            let v = (self.state_fn)(cx, data);
            self.set_value(cx, v);
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &A, event: Event) -> IsUsed {
            if self.on_move.is_none() {
                return Unused;
            }

            match event {
                Event::Command(cmd, code) => {
                    let rev = self.direction.is_reversed();
                    let value = match cmd {
                        Command::Left | Command::Up => match rev {
                            false => self.value - self.step,
                            true => self.value + self.step,
                        },
                        Command::Right | Command::Down => match rev {
                            false => self.value + self.step,
                            true => self.value - self.step,
                        },
                        Command::PageUp | Command::PageDown => {
                            // Generics makes this easier than constructing a literal and multiplying!
                            let mut x = self.step + self.step;
                            x = x + x;
                            x = x + x;
                            x = x + x;
                            match rev == (cmd == Command::PageDown) {
                                false => self.value + x,
                                true => self.value - x,
                            }
                        }
                        Command::Home => self.range.0,
                        Command::End => self.range.1,
                        _ => return Unused,
                    };

                    cx.depress_with_key(&self, code);

                    if self.set_value(cx, value) {
                        if let Some(ref f) = self.on_move {
                            f(cx, data, self.value);
                        }
                    }
                }
                Event::PressStart(press) => {
                    let offset = self.grip.handle_press_on_track(cx, &press);
                    self.apply_grip_offset(cx, data, offset);
                }
                _ => return Unused,
            }
            Used
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &A) {
            if self.on_move.is_none() {
                return;
            }

            match cx.try_pop() {
                Some(GripMsg::PressStart) => cx.set_nav_focus(self.id(), FocusSource::Synthetic),
                Some(GripMsg::PressMove(pos)) => {
                    self.apply_grip_offset(cx, data, pos);
                }
                Some(GripMsg::PressEnd(_)) => (),
                None => {
                    let mut new_value = None;
                    if let Some(SetValueF64(v)) = cx.try_pop() {
                        new_value = v
                            .try_cast_approx()
                            .map_err(|err| log::warn!("Slider failed to handle SetValueF64: {err}"))
                            .ok();
                    } else if let Some(IncrementStep) = cx.try_pop() {
                        new_value = Some(self.value + self.step);
                    } else if let Some(DecrementStep) = cx.try_pop() {
                        new_value = Some(self.value - self.step);
                    }

                    if let Some(value) = new_value
                        && self.set_value(cx, value)
                        && let Some(ref f) = self.on_move
                    {
                        f(cx, data, self.value);
                    }
                }
            }
        }
    }
}
