// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Animation helpers

use crate::Id;
use crate::draw::DrawImpl;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct Config {
    cursor_blink_rate: Duration,
    fade_dur: Duration,
}

/// State holding theme animation data
#[derive(Debug)]
pub struct AnimState<D> {
    c: Config,
    now: Instant, // frame start time
    text_cursor: TextCursor,
    _d: PhantomData<D>,
}

impl<D> AnimState<D> {
    pub fn new(config: &crate::config::ThemeConfig) -> Self {
        let c = Config {
            cursor_blink_rate: config.cursor_blink_rate(),
            fade_dur: config.transition_fade_duration(),
        };
        let now = Instant::now();
        AnimState {
            c,
            now,
            text_cursor: TextCursor {
                widget: 0,
                byte: 0,
                state: false,
                time: now,
            },
            _d: PhantomData,
        }
    }

    pub fn update(&mut self) {
        self.now = Instant::now();
    }

    fn elapsed(&self, time: Instant) -> Option<Duration> {
        if self.now > time { Some(self.now - time) } else { None }
    }
}

#[derive(Clone, Copy, Debug)]
struct TextCursor {
    widget: u64,
    byte: usize,
    state: bool,
    time: Instant,
}
impl<D: DrawImpl> AnimState<D> {
    /// Flashing text cursor: return true to draw
    ///
    /// Assumption: only one widget may draw a text cursor at any time.
    pub fn text_cursor(&mut self, draw: &mut D, id: &Id, byte: usize) -> bool {
        let entry = &mut self.text_cursor;
        let widget = id.to_nzu64().get();
        if entry.widget == widget && entry.byte == byte {
            if entry.time < self.now {
                entry.state = !entry.state;
                entry.time += self.c.cursor_blink_rate;
            }
            draw.animate_at(entry.time);
            entry.state
        } else {
            entry.widget = widget;
            entry.byte = byte;
            entry.state = true;
            entry.time = self.now + self.c.cursor_blink_rate;
            draw.animate_at(entry.time);
            true
        }
    }
}

impl<D: DrawImpl> AnimState<D> {
    /// Fade over a boolean transition
    ///
    /// Normally returns `1.0` if `state` else `0.0`, but within a short time
    /// after a state change will linearly transition between these values.
    pub fn fade_bool(&mut self, draw: &mut D, state: bool, last_change: Option<Instant>) -> f32 {
        if let Some(dur) = last_change.and_then(|inst| self.elapsed(inst))
            && dur < self.c.fade_dur
        {
            draw.animate();
            let f = dur.as_secs_f32() / self.c.fade_dur.as_secs_f32();
            return if state { f } else { 1.0 - f };
        }
        state as u8 as f32
    }
}
