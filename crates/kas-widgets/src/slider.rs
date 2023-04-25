// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Slider` control

use super::{GripMsg, GripPart};
use kas::event::Command;
use kas::prelude::*;
use kas::theme::Feature;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Add, RangeInclusive, Sub};
use std::time::Duration;

/// Requirements on type used by [`Slider`]
///
/// Implementations are provided for standard float and integer types.
pub trait SliderValue:
    Copy + Debug + PartialOrd + Add<Output = Self> + Sub<Output = Self> + 'static
{
    /// The default step size (usually 1)
    fn default_step() -> Self;

    /// Divide self by another instance of this type, returning an `f64`
    ///
    /// Note: in practice, we always have `rhs >= self` and expect the result
    /// to be between 0 and 1.
    fn div_as_f64(self, rhs: Self) -> f64;

    /// Return the result of multiplying self by an `f64` scalar
    ///
    /// Note: the `scalar` is expected to be between 0 and 1, hence this
    /// operation should not produce a value larger than self.
    ///
    /// Also note that this method is not required to preserve precision
    /// (e.g. `u128::mul_64` may drop some low-order bits with large numbers).
    #[must_use]
    fn mul_f64(self, scalar: f64) -> Self;
}

impl SliderValue for f64 {
    fn default_step() -> Self {
        1.0
    }
    fn div_as_f64(self, rhs: Self) -> f64 {
        self / rhs
    }
    fn mul_f64(self, scalar: f64) -> Self {
        self * scalar
    }
}

impl SliderValue for f32 {
    fn default_step() -> Self {
        1.0
    }
    fn div_as_f64(self, rhs: Self) -> f64 {
        self as f64 / rhs as f64
    }
    fn mul_f64(self, scalar: f64) -> Self {
        (self as f64 * scalar) as f32
    }
}

macro_rules! impl_slider_ty {
    ($ty:ty) => {
        impl SliderValue for $ty {
            fn default_step() -> Self {
                1
            }
            fn div_as_f64(self, rhs: Self) -> f64 {
                self as f64 / rhs as f64
            }
            fn mul_f64(self, scalar: f64) -> Self {
                let r = (self as f64 * scalar).round();
                assert!(<$ty>::MIN as f64 <= r && r <= <$ty>::MAX as f64);
                r as $ty
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

/// Implement for [`Duration`]
///
/// The default step size is 1 second.
impl SliderValue for Duration {
    fn default_step() -> Self {
        Duration::from_secs(1)
    }
    fn div_as_f64(self, rhs: Self) -> f64 {
        self.as_secs_f64() / rhs.as_secs_f64()
    }
    fn mul_f64(self, scalar: f64) -> Self {
        self.mul_f64(scalar)
    }
}

impl_scope! {
    /// A slider
    ///
    /// Sliders allow user input of a value from a fixed range.
    #[autoimpl(Debug ignore self.state_fn, self.on_move)]
    #[widget{
        data = A;
        navigable = true;
        hover_highlight = true;
    }]
    pub struct Slider<A, T: SliderValue, D: Directional> {
        core: widget_core!(),
        _data: PhantomData<A>,
        align: AlignPair,
        direction: D,
        // Terminology assumes vertical orientation:
        range: (T, T),
        step: T,
        value: T,
        #[widget(&())]
        grip: GripPart,
        state_fn: Box<dyn Fn(&A) -> T>,
        on_move: Option<Box<dyn Fn(&mut EventCx<A>, T)>>,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a slider
        ///
        /// Values vary within the given `range`. The default step size is
        /// 1 for common types (see [`SliderValue::default_step`]).
        ///
        /// The initial value defaults to the range's
        /// lower bound but may be specified via [`Slider::with_value`].
        #[inline]
        pub fn new(range: RangeInclusive<T>, state_fn: impl Fn(&A) -> T + 'static) -> Self {
            Slider::new_with_direction(range, state_fn, D::default())
        }

        /// Construct a spinner
        ///
        /// - Values vary within the given `range`
        /// - The default step size is 1 for common types (see [`SliderValue::default_step`])
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `message_fn` is emitted when toggled
        #[inline]
        pub fn new_msg<M: std::fmt::Debug + 'static>(
            range: RangeInclusive<T>,
            state_fn: impl Fn(&A) -> T + 'static,
            message_fn: impl Fn(T) -> M + 'static,
        ) -> Self {
            Slider::new(range, state_fn).on_move(move |cx, state| cx.push(message_fn(state)))
        }
    }

    impl Self {
        /// Construct a slider with the given `direction`
        ///
        /// Values vary within the given `range`. The default step size is
        /// 1 for common types (see [`SliderValue::default_step`]).
        ///
        /// The initial value defaults to the range's
        /// lower bound but may be specified via [`Slider::with_value`].
        #[inline]
        pub fn new_with_direction(range: RangeInclusive<T>, state_fn: impl Fn(&A) -> T + 'static, direction: D) -> Self {
            assert!(!range.is_empty());
            let value = *range.start();
            Slider {
                core: Default::default(),
                _data: PhantomData,
                align: Default::default(),
                direction,
                range: range.into_inner(),
                step: T::default_step(),
                value,
                grip: GripPart::new(),
                state_fn: Box::new(state_fn),
                on_move: None,
            }
        }

        /// Set event handler `f`
        ///
        /// This closure is called when the slider is moved.
        #[inline]
        #[must_use]
        pub fn on_move<F>(mut self, f: F) -> Self
        where
            F: Fn(&mut EventCx<A>, T) + 'static,
        {
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

        #[inline]
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        fn clamp_value(&self, value: T) -> T {
            if !(value >= self.range.0) {
                self.range.0
            } else if !(value <= self.range.1) {
                self.range.1
            } else {
                value
            }
        }

        // translate value to offset in local coordinates
        fn offset(&self) -> Offset {
            let a = self.value - self.range.0;
            let b = self.range.1 - self.range.0;
            let max_offset = self.grip.max_offset();
            let mut frac = a.div_as_f64(b);
            assert!((0.0..=1.0).contains(&frac));
            if self.direction.is_reversed() {
                frac = 1.0 - frac;
            }
            match self.direction.is_vertical() {
                false => Offset((max_offset.0 as f64 * frac).cast_floor(), 0),
                true => Offset(0, (max_offset.1 as f64 * frac).cast_floor()),
            }
        }

        fn apply_grip_offset(&mut self, mgr: &mut EventCx<A>, offset: Offset) {
            let b = self.range.1 - self.range.0;
            let max_offset = self.grip.max_offset();
            let mut a = match self.direction.is_vertical() {
                false => b.mul_f64(offset.0 as f64 / max_offset.0 as f64),
                true => b.mul_f64(offset.1 as f64 / max_offset.1 as f64),
            };
            if self.direction.is_reversed() {
                a = b - a;
            }
            let value = self.clamp_value(a + self.range.0);
            if value != self.value {
                self.value = value;
                *mgr |= self.grip.set_offset(self.offset()).1;
                if let Some(ref f) = self.on_move {
                    f(mgr, value);
                }
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.align.set_component(
                axis,
                match axis.is_vertical() == self.direction.is_vertical() {
                    false => axis.align_or_center(),
                    true => axis.align_or_stretch(),
                },
            );
            size_mgr.feature(Feature::Slider(self.direction()), axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            let rect = mgr.align_feature(Feature::Slider(self.direction()), rect, self.align);
            self.core.rect = rect;
            self.grip.set_rect(mgr, rect);
            let mut size = rect.size;
            size.set_component(self.direction, mgr.size_mgr().handle_len());
            let _ = self.grip.set_size_and_offset(size, self.offset());
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            self.grip.find_id(coord).or_else(|| Some(self.id()))
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let dir = self.direction.as_direction();
            draw.slider(self.rect(), &self.grip, dir);
        }
    }

    impl Widget for Self {
        fn update(&mut self, cx: &mut ConfigCx<A>) {
            self.value = self.clamp_value((self.state_fn)(cx.data()));
        }

        fn handle_event(&mut self, cx: &mut EventCx<A>, event: Event) -> Response {
            match event {
                Event::Command(cmd) => {
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
                        _ => return Response::Unused,
                    };

                    let value = self.clamp_value(value);
                    if value != self.value {
                        self.value = value;
                        let _ = self.grip.set_offset(self.offset());
                        cx.redraw(self.id());
                        if let Some(ref f) = self.on_move {
                            f(cx, value);
                        }
                    }
                }
                Event::PressStart { press } => {
                    let offset = self.grip.handle_press_on_track(&mut cx.map(|_| &()), &press);
                    self.apply_grip_offset(cx, offset);
                }
                _ => return Response::Unused,
            }
            Response::Used
        }

        fn handle_message(&mut self, mgr: &mut EventCx<A>) {
            match mgr.try_pop() {
                Some(GripMsg::PressStart) => mgr.set_nav_focus(self.id(), false),
                Some(GripMsg::PressMove(pos)) => {
                    self.apply_grip_offset(mgr, pos);
                }
                _ => (),
            }
        }
    }
}
