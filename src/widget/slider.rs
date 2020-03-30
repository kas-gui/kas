// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Slider` control

use conv::{ApproxFrom, ApproxInto, RoundToNearest};
use std::fmt::Debug;
use std::ops::{Add, Sub};

use super::DragHandle;
use kas::draw::{DrawHandle, SizeHandle};
use kas::event::{Action, Event, Manager, NavKey, Response};
use kas::layout::{AxisInfo, SizeRules, StretchPolicy};
use kas::prelude::*;

pub trait SliderType:
    Copy
    + Debug
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + ApproxInto<f64>
    + ApproxFrom<f64, RoundToNearest>
{
}

impl<
        T: Copy
            + Debug
            + PartialOrd
            + Add<Output = Self>
            + Sub<Output = T>
            + ApproxInto<f64>
            + ApproxFrom<f64, RoundToNearest>,
    > SliderType for T
{
}

/// A slider
///
/// Sliders allow user input of a value from a fixed range.
#[handler(action, msg = T)]
#[widget(config(key_nav = true))]
#[derive(Clone, Debug, Default, Widget)]
pub struct Slider<T: SliderType, D: Directional> {
    #[widget_core]
    core: CoreData,
    direction: D, // TODO: also reversed direction
    // Terminology assumes vertical orientation:
    range: (T, T),
    step: T,
    value: T,
    #[widget]
    handle: DragHandle,
}

impl<T: SliderType, D: Directional + Default> Slider<T, D> {
    /// Construct a slider
    ///
    /// Values vary between the given `min` and `max`. When keyboard navigation
    /// is used, arrow keys will increment the value by `step` and page up/down
    /// keys by `step * 16`.
    ///
    /// The initial value defaults to the range's
    /// lower bound but may be specified via [`Slider::with_value`].
    pub fn new(min: T, max: T, step: T) -> Self {
        Slider::new_with_direction(min, max, step, D::default())
    }
}

impl<T: SliderType, D: Directional> Slider<T, D> {
    /// Construct a slider with the given `direction`
    ///
    /// Values vary between the given `min` and `max`. When keyboard navigation
    /// is used, arrow keys will increment the value by `step` and page up/down
    /// keys by `step * 16`.
    ///
    /// The initial value defaults to the range's
    /// lower bound but may be specified via [`Slider::with_value`].
    #[inline]
    pub fn new_with_direction(min: T, max: T, step: T, direction: D) -> Self {
        assert!(min <= max);
        let value = min;
        Slider {
            core: Default::default(),
            direction,
            range: (min, max),
            step,
            value,
            handle: DragHandle::new(),
        }
    }

    /// Set the initial value
    #[inline]
    pub fn with_value(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    /// Get the current value
    #[inline]
    pub fn value(&self) -> T {
        self.value
    }

    /// Set the value
    ///
    /// Returns true when the value differs from the old value
    pub fn set_value(&mut self, mgr: &mut Manager, mut value: T) -> bool {
        if value < self.range.0 {
            value = self.range.0;
        } else if value > self.range.1 {
            value = self.range.1;
        }
        if value != self.value {
            self.value = value;
            if self.handle.set_offset(self.offset()).1 {
                mgr.redraw(self.handle.id());
            }
            true
        } else {
            false
        }
    }

    // translate value to offset in local coordinates
    fn offset(&self) -> Coord {
        let a: f64 = (self.value - self.range.0).approx_into().unwrap();
        let b: f64 = (self.range.1 - self.range.0).approx_into().unwrap();
        let max_offset = self.handle.max_offset();
        match self.direction.is_vertical() {
            false => Coord((max_offset.0 as f64 * a / b) as i32, 0),
            true => Coord(0, (max_offset.1 as f64 * a / b) as i32),
            // TODO: reversed vert => Coord(0, (max_offset.1 as f64 * (1.0 - a / b)) as i32),
        }
    }

    // true if not equal to old value
    fn set_offset(&mut self, offset: Coord) -> bool {
        let b: f64 = (self.range.1 - self.range.0).approx_into().unwrap();
        let max_offset = self.handle.max_offset();
        let a = match self.direction.is_vertical() {
            false => b * offset.0 as f64 / max_offset.0 as f64,
            true => b * offset.1 as f64 / max_offset.1 as f64,
            // TODO: reversed vert => b * (1.0 - offset.1 as f64 / max_offset.1 as f64),
        };
        let value = T::approx_from(a).unwrap() + self.range.0;
        let value = if !(value >= self.range.0) {
            self.range.0
        } else if !(value <= self.range.1) {
            self.range.1
        } else {
            value
        };
        if value != self.value {
            self.value = value;
            return true;
        }
        false
    }
}

impl<T: SliderType, D: Directional> Layout for Slider<T, D> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let (size, min_len) = size_handle.slider();
        let margins = (0, 0);
        if self.direction.is_vertical() == axis.is_vertical() {
            SizeRules::new(min_len, min_len, margins, StretchPolicy::HighUtility)
        } else {
            SizeRules::fixed(size.1, margins)
        }
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        let (size, _) = size_handle.slider();
        let mut size = size.min(rect.size);
        if self.direction.is_vertical() {
            size = size.transpose();
        }
        self.core.rect = rect;
        self.handle.set_rect(size_handle, rect, align);
        self.handle.set_size_and_offset(size, self.offset());
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if self.handle.rect().contains(coord) {
            Some(self.handle.id())
        } else {
            Some(self.id())
        }
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        // Depending on whether we get the highlight state for self.id() or
        // self.handle.id() we can highlight when over the slider or just the
        // handle. But for key-nav, we want highlight-state of self.
        let hl = mgr.highlight_state(self.id());

        let dir = self.direction.as_direction();
        draw_handle.slider(self.core.rect, self.handle.rect(), dir, hl);
    }
}

impl<T: SliderType, D: Directional> event::EventHandler for Slider<T, D> {
    fn event(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        let offset = if id <= self.handle.id() {
            match self.handle.event(mgr, id, event).try_into() {
                Ok(res) => return res,
                Err(offset) => offset,
            }
        } else {
            match event {
                Event::Action(Action::NavKey(key)) => {
                    // Generics makes this easier than constructing a literal and multiplying!
                    let sixteen_of = |mut x: T| -> T {
                        x = x + x;
                        x = x + x;
                        x = x + x;
                        x + x
                    };

                    let v = match key {
                        NavKey::Left | NavKey::Up => self.value - self.step,
                        NavKey::Right | NavKey::Down => self.value + self.step,
                        NavKey::PageUp => self.value - sixteen_of(self.step),
                        NavKey::PageDown => self.value + sixteen_of(self.step),
                        NavKey::Home => self.range.0,
                        NavKey::End => self.range.1,
                        // _ => return Response::Unhandled(event),
                    };
                    return if self.set_value(mgr, v) {
                        Response::Msg(self.value)
                    } else {
                        Response::None
                    };
                }
                Event::PressStart { source, coord, .. } => {
                    self.handle.handle_press_on_track(mgr, source, coord)
                }
                ev @ _ => return Response::Unhandled(ev),
            }
        };

        let r = if self.set_offset(offset) {
            Response::Msg(self.value)
        } else {
            Response::None
        };
        self.handle.set_offset(self.offset());
        r
    }
}
