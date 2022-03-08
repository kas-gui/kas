// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Animation helpers

use kas::draw::DrawImpl;
use kas::WidgetId;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(8);

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
    time_next_gc: Instant,
    text_cursor: HashMap<u64, TextCursor>,
    fade_bool: HashMap<u64, FadeBool>,
    _d: PhantomData<D>,
}

impl<D> AnimState<D> {
    pub fn new(config: &super::Config) -> Self {
        let c = Config {
            cursor_blink_rate: config.cursor_blink_rate(),
            fade_dur: config.transition_fade_duration(),
        };
        let now = Instant::now();
        AnimState {
            c,
            now,
            time_next_gc: now + TIMEOUT,
            text_cursor: Default::default(),
            fade_bool: Default::default(),
            _d: PhantomData,
        }
    }

    pub fn update(&mut self) {
        self.now = Instant::now();
        if self.time_next_gc <= self.now {
            self.garbage_collect();
        }
    }

    fn garbage_collect(&mut self) {
        self.time_next_gc = self.now + TIMEOUT;
        let old = self.now - TIMEOUT;
        self.text_cursor.retain(|_, v| v.time >= old);
        self.fade_bool.retain(|_, v| v.time >= old);
    }
}

#[derive(Clone, Copy, Debug)]
struct TextCursor {
    byte: usize,
    state: bool,
    time: Instant,
}
impl<D: DrawImpl> AnimState<D> {
    /// Flashing text cursor: return true to draw
    pub fn text_cursor(&mut self, draw: &mut D, id: &WidgetId, byte: usize) -> bool {
        match self.text_cursor.entry(id.as_u64()) {
            Entry::Occupied(entry) if entry.get().byte == byte => {
                let entry = entry.into_mut();
                if entry.time < self.now {
                    entry.state = !entry.state;
                    entry.time += self.c.cursor_blink_rate;
                }
                draw.animate_at(entry.time);
                entry.state
            }
            entry => {
                let time = self.now + self.c.cursor_blink_rate;
                let state = true;
                let v = TextCursor { byte, state, time };
                match entry {
                    Entry::Occupied(mut entry) => {
                        entry.insert(v);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(v);
                    }
                }
                draw.animate_at(time);
                true
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FadeBool {
    state: bool,
    time: Instant,
    time_end: Instant,
}
impl<D: DrawImpl> AnimState<D> {
    /// Fade over a boolean transition
    ///
    /// Normally returns `1.0` if `state` else `0.0`, but within a short time
    /// after a state change will linearly transition between these values.
    #[inline]
    pub fn fade_bool(&mut self, draw: &mut D, id: &WidgetId, state: bool) -> f32 {
        1.0 - self.fade_bool_1m(draw, id, state)
    }

    /// `1.0 - self.fade_bool(..)`
    pub fn fade_bool_1m(&mut self, draw: &mut D, id: &WidgetId, state: bool) -> f32 {
        let mut out_state = state;
        match self.fade_bool.entry(id.as_u64()) {
            Entry::Occupied(entry) => {
                let entry = entry.into_mut();
                entry.time = self.now;
                if entry.state != state {
                    out_state = entry.state;
                    entry.state = state;
                    entry.time_end = self.now + self.c.fade_dur;
                    draw.animate();
                } else if self.now < entry.time_end {
                    let rem = entry.time_end - self.now;
                    let mut f = rem.as_secs_f32() / self.c.fade_dur.as_secs_f32();
                    if !state {
                        f = 1.0 - f;
                    }
                    draw.animate();
                    return f;
                }
            }
            entry => {
                let v = FadeBool {
                    state,
                    time: self.now,
                    time_end: self.now,
                };
                match entry {
                    Entry::Occupied(mut entry) => {
                        entry.insert(v);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(v);
                    }
                }
            }
        }
        !out_state as u8 as f32
    }
}
