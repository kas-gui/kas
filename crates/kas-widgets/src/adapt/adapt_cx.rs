// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapted contexts

use kas::prelude::*;
use std::time::Duration;

/// An [`EventCx`] with embedded [`Id`] and input data
///
/// NOTE: this is a temporary design: it may be expanded or integrated with
/// `EventCx` in the future.
#[autoimpl(Deref, DerefMut using self.cx)]
pub struct AdaptEventCx<'a: 'b, 'b, A> {
    cx: &'b mut EventCx<'a>,
    id: Id,
    data: &'b A,
}

impl<'a: 'b, 'b, A> AdaptEventCx<'a, 'b, A> {
    /// Construct
    #[inline]
    pub fn new(cx: &'b mut EventCx<'a>, id: Id, data: &'b A) -> Self {
        AdaptEventCx { cx, id, data }
    }

    /// Access input data
    #[inline]
    pub fn data(&'b self) -> &'b A {
        self.data
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
    /// This widget will receive an update for timer `timer_id` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid except from a timer
    /// handler where it might cause an infinite loop.
    ///
    /// Multiple timer requests with the same `timer_id` are merged
    /// (choosing the earliest time).
    #[inline]
    pub fn request_timer(&mut self, timer_id: u64, delay: Duration) {
        self.cx.request_timer(self.id.clone(), timer_id, delay);
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

    /// Enable `alt_bypass` for layer
    ///
    /// This may be called by a child widget during configure to enable or
    /// disable alt-bypass for the access-key layer containing its access keys.
    /// This allows access keys to be used as shortcuts without the Alt
    /// key held. See also [`EventState::new_access_layer`].
    #[inline]
    pub fn enable_alt_bypass(&mut self, alt_bypass: bool) {
        self.cx.enable_alt_bypass(&self.id, alt_bypass);
    }

    /// Schedule a timed update
    ///
    /// This widget will receive an update for timer `timer_id` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid except from a timer
    /// handler where it might cause an infinite loop.
    ///
    /// Multiple timer requests with the same `timer_id` are merged
    /// (choosing the earliest time).
    #[inline]
    pub fn request_timer(&mut self, timer_id: u64, delay: Duration) {
        self.cx.request_timer(self.id.clone(), timer_id, delay);
    }
}
