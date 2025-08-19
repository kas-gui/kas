// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapted contexts

use kas::event::TimerHandle;
use kas::prelude::*;
use std::time::Duration;

/// An [`EventCx`] with embedded [`Id`] and input data
///
/// NOTE: this is a temporary design: it may be expanded or integrated with
/// `EventCx` in the future.
#[autoimpl(Deref, DerefMut using self.cx)]
pub struct AdaptEventCx<'a: 'b, 'b> {
    cx: &'b mut EventCx<'a>,
    id: Id,
}

impl<'a: 'b, 'b> AdaptEventCx<'a, 'b> {
    /// Construct
    #[inline]
    pub fn new(cx: &'b mut EventCx<'a>, id: Id) -> Self {
        AdaptEventCx { cx, id }
    }

    /// Check whether this widget is disabled
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.cx.is_disabled(&self.id)
    }

    /// Set/unset disabled status for this widget
    #[inline]
    pub fn set_disabled(&mut self, state: bool) {
        self.cx.set_disabled(self.id.clone(), state);
    }

    /// Schedule a timed update
    ///
    /// Widget updates may be used for delayed action. For animation, prefer to
    /// use [`Draw::animate`](kas::draw::Draw::animate) or
    /// [`Self::request_frame_timer`].
    ///
    /// This widget will receive an update for timer `handle` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid except from a timer
    /// handler where it may cause an infinite loop.
    ///
    /// Multiple timer requests with the same `id` and `handle` are merged
    /// (see [`TimerHandle`] documentation).
    #[inline]
    pub fn request_timer(&mut self, handle: TimerHandle, delay: Duration) {
        self.cx.request_timer(self.id.clone(), handle, delay);
    }

    /// Schedule a frame timer update
    ///
    /// Widget `id` will receive [`Event::Timer`] with this `handle` either
    /// before or soon after the next frame is drawn.
    ///
    /// This may be useful for animations which mutate widget state. Animations
    /// which don't mutate widget state may use
    /// [`Draw::animate`](kas::draw::Draw::animate) instead.
    ///
    /// It is expected that `handle.earliest() == true` (style guide).
    #[inline]
    pub fn request_frame_timer(&mut self, handle: TimerHandle) {
        self.cx.request_frame_timer(self.id.clone(), handle);
    }
}

/// A [`ConfigCx`] with embedded [`Id`]
///
/// NOTE: this is a temporary design: it may be expanded or integrated with
/// `ConfigCx` in the future.
#[autoimpl(Deref, DerefMut using self.cx)]
pub struct AdaptConfigCx<'a: 'b, 'b> {
    cx: &'b mut ConfigCx<'a>,
    id: Id,
}

impl<'a: 'b, 'b> AdaptConfigCx<'a, 'b> {
    /// Construct
    #[inline]
    pub fn new(cx: &'b mut ConfigCx<'a>, id: Id) -> Self {
        AdaptConfigCx { cx, id }
    }

    /// Check whether this widget is disabled
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.cx.is_disabled(&self.id)
    }

    /// Set/unset disabled status for this widget
    #[inline]
    pub fn set_disabled(&mut self, state: bool) {
        self.cx.set_disabled(self.id.clone(), state);
    }

    /// Schedule a timed update
    ///
    /// Widget updates may be used for delayed action. For animation, prefer to
    /// use [`Draw::animate`](kas::draw::Draw::animate) or
    /// [`Self::request_frame_timer`].
    ///
    /// This widget will receive an update for timer `handle` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid except from a timer
    /// handler where it may cause an infinite loop.
    ///
    /// Multiple timer requests with the same `id` and `handle` are merged
    /// (see [`TimerHandle`] documentation).
    #[inline]
    pub fn request_timer(&mut self, handle: TimerHandle, delay: Duration) {
        self.cx.request_timer(self.id.clone(), handle, delay);
    }

    /// Schedule a frame timer update
    ///
    /// Widget `id` will receive [`Event::Timer`] with this `handle` either
    /// before or soon after the next frame is drawn.
    ///
    /// This may be useful for animations which mutate widget state. Animations
    /// which don't mutate widget state may use
    /// [`Draw::animate`](kas::draw::Draw::animate) instead.
    ///
    /// It is expected that `handle.earliest() == true` (style guide).
    #[inline]
    pub fn request_frame_timer(&mut self, handle: TimerHandle) {
        self.cx.request_frame_timer(self.id.clone(), handle);
    }
}
