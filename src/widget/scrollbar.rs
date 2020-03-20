// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `ScrollBar` control

use std::fmt::Debug;

use super::DragHandle;
use crate::draw::{DrawHandle, SizeHandle};
use crate::event::{self, Event, Manager, Response};
use crate::geom::*;
use crate::layout::{AxisInfo, SizeRules, StretchPolicy};
use crate::macros::Widget;
use crate::{AlignHints, CoreData, Directional, Layout, WidgetCore, WidgetId};

/// A scroll bar
///
/// Scroll bars allow user-input of a value between 0 and a defined maximum,
/// and allow the size of the handle to be specified.
#[widget_config]
#[handler(action, msg = u32)]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollBar<D: Directional> {
    #[widget_core]
    core: CoreData,
    direction: D,
    // Terminology assumes vertical orientation:
    min_handle_len: u32,
    handle_len: u32,
    handle_value: u32, // contract: > 0
    max_value: u32,
    value: u32,
    #[widget]
    handle: DragHandle,
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
            handle: DragHandle::new(),
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

    /// Set the initial value
    #[inline]
    pub fn with_value(mut self, value: u32) -> Self {
        self.value = value;
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
    ///
    /// Returns true if a redraw is required.
    pub fn set_limits(&mut self, max_value: u32, handle_value: u32) -> bool {
        // We should gracefully handle zero, though appearance may be wrong.
        self.handle_value = handle_value.max(1);

        self.max_value = max_value;
        self.value = self.value.min(self.max_value);
        self.update_handle()
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
            if self.handle.set_offset(self.offset()).1 {
                mgr.redraw(self.handle.id());
            }
        }
    }

    #[inline]
    fn len(&self) -> u32 {
        match self.direction.is_vertical() {
            false => self.core.rect.size.0,
            true => self.core.rect.size.1,
        }
    }

    fn update_handle(&mut self) -> bool {
        let len = self.len();
        let total = self.max_value as u64 + self.handle_value as u64;
        let handle_len = self.handle_value as u64 * len as u64 / total;
        self.handle_len = (handle_len as u32).max(self.min_handle_len).min(len);
        let mut size = self.core.rect.size;
        if self.direction.is_horizontal() {
            size.0 = self.handle_len;
        } else {
            size.1 = self.handle_len;
        }
        self.handle.set_size_and_offset(size, self.offset())
    }

    // translate value to offset in local coordinates
    fn offset(&self) -> Coord {
        let len = self.len() - self.handle_len;
        let lhs = self.value as u64 * len as u64;
        let rhs = self.max_value as u64;
        let pos = if rhs == 0 {
            0
        } else {
            (((lhs + (rhs / 2)) / rhs) as u32).min(len) as i32
        };
        match self.direction.is_vertical() {
            false => Coord(pos, 0),
            true => Coord(0, pos),
        }
    }

    // true if not equal to old value
    fn set_offset(&mut self, offset: Coord) -> bool {
        let offset = match self.direction.is_vertical() {
            false => offset.0,
            true => offset.1,
        };

        let len = self.len() - self.handle_len;
        let lhs = offset as u64 * self.max_value as u64;
        let rhs = len as u64;
        if rhs == 0 {
            debug_assert_eq!(self.value, 0);
            return false;
        }
        let value = ((lhs + (rhs / 2)) / rhs) as u32;
        let value = value.min(self.max_value);
        if value != self.value {
            self.value = value;
            return true;
        }
        false
    }
}

impl<D: Directional> Layout for ScrollBar<D> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let (size, min_len) = size_handle.scrollbar();
        let margins = (0, 0);
        if self.direction.is_vertical() == axis.is_vertical() {
            SizeRules::new(min_len, min_len, margins, StretchPolicy::HighUtility)
        } else {
            SizeRules::fixed(size.1, margins)
        }
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        let (size, _) = size_handle.scrollbar();
        self.min_handle_len = size.0;
        self.core.rect = rect;
        self.handle.set_rect(size_handle, rect, align);
        self.update_handle();
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if self.handle.rect().contains(coord) {
            Some(self.handle.id())
        } else {
            Some(self.id())
        }
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        let dir = self.direction.as_direction();
        let hl = mgr.highlight_state(self.handle.id());
        draw_handle.scrollbar(self.core.rect, self.handle.rect(), dir, hl);
    }
}

impl<D: Directional> event::EventHandler for ScrollBar<D> {
    fn event(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        let offset = if id <= self.handle.id() {
            match self.handle.event(mgr, id, event).try_into() {
                Ok(res) => return res,
                Err(offset) => offset,
            }
        } else {
            match event {
                Event::PressStart { source, coord, .. } => {
                    self.handle.handle_press_on_track(mgr, source, coord)
                }
                ev @ _ => return Response::Unhandled(ev),
            }
        };

        if self.set_offset(offset) {
            Response::Msg(self.value)
        } else {
            Response::None
        }
    }
}
