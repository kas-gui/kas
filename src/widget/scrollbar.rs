// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `ScrollBar` control

use std::fmt::Debug;

use crate::event::{Event, Handler, Manager, PressSource, Response};
use crate::geom::Rect;
use crate::layout::{AxisInfo, SizeRules, StretchPolicy};
use crate::macros::Widget;
use crate::theme::{DrawHandle, SizeHandle};
use crate::{CoreData, Directional, Layout, WidgetCore, WidgetId};

/// A scroll bar
///
/// Scroll bars allow user-input of a value between 0 and a defined maximum,
/// and allow the size of the handle to be specified.
#[widget]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollBar<D: Directional> {
    #[core]
    core: CoreData,
    direction: D,
    // Terminology assumes vertical orientation:
    min_handle_len: u32,
    handle_len: u32,
    handle_value: u32, // contract: > 0
    max_value: u32,
    value: u32,
    press_source: Option<PressSource>,
    press_offset: i32,
}

impl<D: Directional + Default> ScrollBar<D> {
    /// Construct a scroll bar
    ///
    /// Default values are assumed for all parameters.
    pub fn new() -> Self {
        ScrollBar::new_with_direction(D::default())
    }
}

impl<D: Directional> ScrollBar<D> {
    /// Construct a scroll bar with the given direction
    ///
    /// Default values are assumed for all parameters.
    #[inline]
    pub fn new_with_direction(direction: D) -> Self {
        ScrollBar {
            core: Default::default(),
            direction,
            min_handle_len: 0,
            handle_len: 0,
            handle_value: 1,
            max_value: 0,
            value: 0,
            press_source: None,
            press_offset: 0,
        }
    }

    /// Set the page length
    ///
    /// See [`ScrollBar::set_limits`].
    #[inline]
    pub fn with_limits(mut self, max_value: u32, handle_value: u32) -> Self {
        self.set_limits(max_value, handle_value);
        self
    }

    /// Set the page limits
    ///
    /// The `max_value` parameter specifies the maximum possible value.
    /// (The minimum is always 0.) For a scroll region, this should correspond
    /// to the maximum possible offset.
    ///
    /// The `handle_value` parameter specifies the size of the handle relative to
    /// the maximum value: the handle size relative to the length of the scroll
    /// bar is `handle_value / (max_value + handle_value)`. For a scroll region,
    /// this should correspond to the size of the visible region.
    /// The minimum value is 1.
    ///
    /// The choice of units is not important (e.g. can be pixels or lines),
    /// so long as both parameters use the same units.
    pub fn set_limits(&mut self, max_value: u32, handle_value: u32) {
        // We should gracefully handle zero, though appearance may be wrong.
        self.handle_value = handle_value.max(1);

        self.max_value = max_value;
        self.value = self.value.min(self.max_value);
        self.update_handle();
    }

    /// Get the current value
    #[inline]
    pub fn value(&self) -> u32 {
        self.value
    }

    /// Set the value
    pub fn set_value(&mut self, mgr: &mut Manager, value: u32) {
        let value = value.min(self.max_value);
        if value != self.value {
            self.value = value;
            mgr.redraw(self.id());
        }
    }

    #[inline]
    fn len(&self) -> u32 {
        match self.direction.is_vertical() {
            false => self.core.rect.size.0,
            true => self.core.rect.size.1,
        }
    }

    fn update_handle(&mut self) {
        let len = self.len();
        let total = self.max_value as u64 + self.handle_value as u64;
        let handle_len = self.handle_value as u64 * len as u64 / total;
        self.handle_len = (handle_len as u32).max(self.min_handle_len).min(len);
        self.value = self.value.min(self.max_value);
    }

    // translate value to position in local coordinates
    fn position(&self) -> u32 {
        let len = self.len() - self.handle_len;
        let lhs = self.value as u64 * len as u64;
        let rhs = self.max_value as u64;
        if rhs == 0 {
            return 0;
        }
        let pos = ((lhs + (rhs / 2)) / rhs) as u32;
        pos.min(len)
    }

    // true if not equal to old value
    fn set_position(&mut self, mgr: &mut Manager, position: u32) -> bool {
        let len = self.len() - self.handle_len;
        let lhs = position as u64 * self.max_value as u64;
        let rhs = len as u64;
        if rhs == 0 {
            debug_assert_eq!(self.value, 0);
            return false;
        }
        let value = ((lhs + (rhs / 2)) / rhs) as u32;
        let value = value.min(self.max_value);
        if value != self.value {
            self.value = value;
            mgr.redraw(self.id());
            return true;
        }
        false
    }
}

impl<D: Directional> Layout for ScrollBar<D> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let (thickness, _, min_len) = size_handle.scrollbar();
        let rules = if self.direction.is_vertical() == axis.is_vertical() {
            SizeRules::new(min_len, min_len, StretchPolicy::LowUtility)
        } else {
            SizeRules::fixed(thickness)
        };
        if axis.is_horizontal() {
            self.core_data_mut().rect.size.0 = rules.ideal_size();
        } else {
            self.core_data_mut().rect.size.1 = rules.ideal_size();
        }
        rules
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect) {
        let (_, min_handle_len, _) = size_handle.scrollbar();
        self.min_handle_len = min_handle_len;
        self.core.rect = rect;
        self.update_handle();
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &Manager) {
        let dir = self.direction.as_direction();
        let hl = mgr.highlight_state(self.id());
        draw_handle.scrollbar(self.core.rect, dir, self.handle_len, self.position(), hl);
    }
}

impl<D: Directional> Handler for ScrollBar<D> {
    type Msg = u32;

    fn handle(&mut self, mgr: &mut Manager, _: WidgetId, event: Event) -> Response<Self::Msg> {
        match event {
            Event::PressStart { source, coord, .. } => {
                if !mgr.request_press_grab(source, self, coord) {
                    return Response::None;
                }
                // Interacting with a scrollbar with multiple presses
                // does not make sense. Any other gets aborted.
                self.press_source = Some(source);

                // Event delivery implies coord is over the scrollbar.
                let (pointer, offset) = match self.direction.is_vertical() {
                    false => (coord.0, self.core.rect.pos.0),
                    true => (coord.1, self.core.rect.pos.1),
                };
                let position = self.position() as i32;
                let h_start = offset + position;

                if pointer >= h_start && pointer < h_start + self.handle_len as i32 {
                    // coord is on the scroll handle
                    self.press_offset = position - pointer;
                    Response::None
                } else {
                    // coord is not on the handle; we move the bar immediately
                    self.press_offset = -offset - (self.handle_len / 2) as i32;
                    let position = (pointer + self.press_offset).max(0) as u32;
                    let moved = self.set_position(mgr, position);
                    debug_assert!(moved);
                    mgr.redraw(self.id());
                    Response::Msg(self.value)
                }
            }
            Event::PressMove { source, coord, .. } if Some(source) == self.press_source => {
                let pointer = match self.direction.is_vertical() {
                    false => coord.0,
                    true => coord.1,
                };
                let position = (pointer + self.press_offset).max(0) as u32;
                if self.set_position(mgr, position) {
                    mgr.redraw(self.id());
                    Response::Msg(self.value)
                } else {
                    Response::None
                }
            }
            Event::PressEnd { source, .. } if Some(source) == self.press_source => {
                self.press_source = None;
                Response::None
            }
            e @ _ => Manager::handle_generic(self, mgr, e),
        }
    }
}
