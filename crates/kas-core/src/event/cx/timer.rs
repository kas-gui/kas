// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context: timers

use super::{EventCx, EventState};
use crate::{Id, Node, TileExt, event::Event, geom::Size};
use std::time::{Duration, Instant};

/// A timer handle
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TimerHandle(i64);
impl TimerHandle {
    /// Construct a new handle
    ///
    /// The code must be positive. If a widget uses multiple timers, each must
    /// have a unique code.
    ///
    /// When a timer update is requested multiple times before delivery using
    /// the same `TimerHandle`, these requests are merged, choosing the
    /// earliest time if `earliest`, otherwise the latest time.
    pub const fn new(code: i64, earliest: bool) -> Self {
        assert!(code >= 0);
        if earliest {
            TimerHandle(-code - 1)
        } else {
            TimerHandle(code)
        }
    }

    /// Check whether this timer chooses the earliest time when merging
    pub fn earliest(self) -> bool {
        self.0 < 0
    }
}

impl EventState {
    /// Get the next resume time
    pub(crate) fn next_resume(&self) -> Option<Instant> {
        self.time_updates.last().map(|time| time.0)
    }

    pub(crate) fn need_frame_update(&self) -> bool {
        self.need_frame_update || !self.frame_updates.is_empty() || !self.fut_messages.is_empty()
    }

    /// Schedule a timed update
    ///
    /// Widget updates may be used for delayed action. For animation, prefer to
    /// use [`Draw::animate`](crate::draw::Draw::animate) or
    /// [`Self::request_frame_timer`].
    ///
    /// Widget `id` will receive [`Event::Timer`] with this `handle` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid, except from an
    /// [`Event::Timer`] handler (where it may cause an infinite loop).
    ///
    /// Multiple timer requests with the same `id` and `handle` are merged
    /// (see [`TimerHandle`] documentation).
    pub fn request_timer(&mut self, id: Id, handle: TimerHandle, delay: Duration) {
        let time = Instant::now() + delay;
        if let Some(row) = self
            .time_updates
            .iter_mut()
            .find(|row| row.1 == id && row.2 == handle)
        {
            let earliest = handle.earliest();
            if earliest && row.0 <= time || !earliest && row.0 >= time {
                return;
            }

            row.0 = time;
        } else {
            log::trace!(
                target: "kas_core::event",
                "request_timer: update {id} at now+{}ms",
                delay.as_millis()
            );
            self.time_updates.push((time, id, handle));
        }

        self.time_updates.sort_by(|a, b| b.0.cmp(&a.0)); // reverse sort
    }

    /// Schedule a frame timer update
    ///
    /// Widget `id` will receive [`Event::Timer`] with this `handle` either
    /// before or soon after the next frame is drawn.
    ///
    /// This may be useful for animations which mutate widget state. Animations
    /// which don't mutate widget state may use
    /// [`Draw::animate`](crate::draw::Draw::animate) instead.
    ///
    /// It is expected that `handle.earliest() == true` (style guide).
    pub fn request_frame_timer(&mut self, id: Id, handle: TimerHandle) {
        debug_assert!(handle.earliest());
        self.frame_updates.insert((id, handle));
    }
}

impl<'a> EventCx<'a> {
    /// Pre-draw / pre-sleep
    ///
    /// This method should be called once per frame as well as after the last
    /// frame before a long sleep.
    pub(crate) fn frame_update(&mut self, mut widget: Node<'_>) {
        self.need_frame_update = false;
        log::trace!(target: "kas_core::event", "Processing frame update");
        if let Some((target, affine)) = self.mouse.frame_update() {
            self.send_event(widget.re(), target, Event::Pan(affine));
        }
        self.touch_frame_update(widget.re());

        let frame_updates = std::mem::take(&mut self.frame_updates);
        for (id, handle) in frame_updates.into_iter() {
            self.send_event(widget.re(), id, Event::Timer(handle));
        }

        // Set IME cursor area, if moved.
        if self.ime.is_some()
            && let Some(target) = self.sel_focus.as_ref()
            && let Some((mut rect, translation)) = widget.as_tile().find_tile_rect(target)
        {
            if self.ime_cursor_area.size != Size::ZERO {
                rect = self.ime_cursor_area;
            }
            rect += translation;
            if rect != self.last_ime_rect {
                self.window.set_ime_cursor_area(rect);
                self.last_ime_rect = rect;
            }
        }
    }

    /// Update widgets due to timer
    pub(crate) fn update_timer(&mut self, mut widget: Node<'_>) {
        let now = Instant::now();

        // assumption: time_updates are sorted in reverse order
        while !self.time_updates.is_empty() {
            if self.time_updates.last().unwrap().0 > now {
                break;
            }

            let update = self.time_updates.pop().unwrap();
            self.send_event(widget.re(), update.1, Event::Timer(update.2));
        }

        self.time_updates.sort_by(|a, b| b.0.cmp(&a.0)); // reverse sort
    }
}
