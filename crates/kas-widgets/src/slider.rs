// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Slider` control

use std::fmt::Debug;
use std::ops::{Add, RangeInclusive, Sub};
use std::time::Duration;

use super::DragHandle;
use kas::event::{Command, MsgPressFocus, Scroll};
use kas::prelude::*;
use kas::theme::Feature;

/// Requirements on type used by [`Slider`]
pub trait SliderType:
    Copy + Debug + PartialOrd + Add<Output = Self> + Sub<Output = Self> + 'static
{
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

impl SliderType for f64 {
    fn div_as_f64(self, rhs: Self) -> f64 {
        self / rhs
    }
    fn mul_f64(self, scalar: f64) -> Self {
        self * scalar
    }
}

impl SliderType for f32 {
    fn div_as_f64(self, rhs: Self) -> f64 {
        self as f64 / rhs as f64
    }
    fn mul_f64(self, scalar: f64) -> Self {
        (self as f64 * scalar) as f32
    }
}

macro_rules! impl_slider_ty {
    ($ty:ty) => {
        impl SliderType for $ty {
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

impl SliderType for Duration {
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
    ///
    /// # Messages
    ///
    /// On value change, pushes a value of type `T`.
    #[derive(Clone, Debug)]
    #[widget{
        key_nav = true;
        hover_highlight = true;
    }]
    pub struct Slider<T: SliderType, D: Directional> {
        core: widget_core!(),
        direction: D,
        // Terminology assumes vertical orientation:
        range: RangeInclusive<T>,
        step: T,
        value: T,
        #[widget]
        handle: DragHandle,
    }

    impl Self where D: Default {
        /// Construct a slider
        ///
        /// Values vary within the given `range`. When keyboard navigation
        /// is used, arrow keys will increment the value by `step` and page up/down
        /// keys by `step * 16`.
        ///
        /// The initial value defaults to the range's
        /// lower bound but may be specified via [`Slider::with_value`].
        #[inline]
        pub fn new(range: RangeInclusive<T>, step: T) -> Self {
            Slider::new_with_direction(range, step, D::default())
        }
    }

    impl Self {
        /// Construct a slider with the given `direction`
        ///
        /// Values vary within the given `range`. When keyboard navigation
        /// is used, arrow keys will increment the value by `step` and page up/down
        /// keys by `step * 16`.
        ///
        /// The initial value defaults to the range's
        /// lower bound but may be specified via [`Slider::with_value`].
        #[inline]
        pub fn new_with_direction(range: RangeInclusive<T>, step: T, direction: D) -> Self {
            assert!(!range.is_empty());
            let value = *range.start();
            Slider {
                core: Default::default(),
                direction,
                range,
                step,
                value,
                handle: DragHandle::new(),
            }
        }

        /// Get the slider's direction
        #[inline]
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Set the initial value
        #[inline]
        #[must_use]
        pub fn with_value(mut self, value: T) -> Self {
            self.value = self.clamp_value(value);
            self
        }

        /// Get the current value
        #[inline]
        pub fn value(&self) -> T {
            self.value
        }

        #[inline]
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        fn clamp_value(&self, value: T) -> T {
            if !(value >= *self.range.start()) {
                *self.range.start()
            } else if !(value <= *self.range.end()) {
                *self.range.end()
            } else {
                value
            }
        }

        /// Set the value
        ///
        /// Returns [`TkAction::REDRAW`] if a redraw is required.
        pub fn set_value(&mut self, value: T) -> TkAction {
            let value = self.clamp_value(value);
            if value == self.value {
                TkAction::empty()
            } else {
                self.value = value;
                self.handle.set_offset(self.offset()).1
            }
        }

        // translate value to offset in local coordinates
        fn offset(&self) -> Offset {
            let a = self.value - *self.range.start();
            let b = *self.range.end() - *self.range.start();
            let max_offset = self.handle.max_offset();
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

        fn set_offset_and_push_msg(&mut self, mgr: &mut EventMgr, offset: Offset) {
            let b = *self.range.end() - *self.range.start();
            let max_offset = self.handle.max_offset();
            let mut a = match self.direction.is_vertical() {
                false => b.mul_f64(offset.0 as f64 / max_offset.0 as f64),
                true => b.mul_f64(offset.1 as f64 / max_offset.1 as f64),
            };
            if self.direction.is_reversed() {
                a = b - a;
            }
            let value = self.clamp_value(a + *self.range.start());
            if value != self.value {
                self.value = value;
                *mgr |= self.handle.set_offset(self.offset()).1;
                mgr.push_msg(self.value);
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            size_mgr.feature(Feature::Slider(self.direction()), axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            self.handle.set_rect(mgr, rect, align);
            let dir = Direction::Right;
            let handle_size = mgr.size_mgr().feature(Feature::Slider(dir), dir).min_size();
            let mut size = rect.size;
            if self.direction.is_horizontal() {
                size.0 = size.0.min(handle_size);
            } else {
                size.1 = size.1.min(handle_size);
            }
            let _ = self.handle.set_size_and_offset(size, self.offset());
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            self.handle.find_id(coord).or(Some(self.id()))
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let dir = self.direction.as_direction();
            draw.slider(self.rect(), &self.handle, dir);
        }
    }

    impl Widget for Self {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::NavFocus(true) => {
                    mgr.set_scroll(Scroll::Rect(self.rect()));
                }
                Event::Command(cmd) => {
                    let rev = self.direction.is_reversed();
                    let v = match cmd {
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
                        Command::Home => *self.range.start(),
                        Command::End => *self.range.end(),
                        _ => return Response::Unused,
                    };
                    let action = self.set_value(v);
                    if !action.is_empty() {
                        mgr.send_action(action);
                        mgr.push_msg(self.value);
                    }
                }
                Event::PressStart { source, coord, .. } => {
                    let offset = self.handle.handle_press_on_track(mgr, source, coord);
                    self.set_offset_and_push_msg(mgr, offset);
                }
                _ => return Response::Unused,
            }
            Response::Used
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(MsgPressFocus) = mgr.try_pop_msg() {
                mgr.set_nav_focus(self.id(), false);
            } else if let Some(offset) = mgr.try_pop_msg() {
                self.set_offset_and_push_msg(mgr, offset);
            }
        }
    }
}
