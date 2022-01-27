// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Animation helpers

use kas::draw::DrawImpl;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug)]
struct Config {
    cursor_blink_rate: Duration,
}

/// State holding theme animation data
#[derive(Debug)]
pub struct AnimState<D> {
    c: Config,
    now: Instant, // frame start time
    time_next_gc: Instant,
    text_cursor: HashMap<u64, TextCursor>,
    _d: PhantomData<D>,
}

impl<D> AnimState<D> {
    pub fn new(config: &super::Config) -> Self {
        let c = Config {
            cursor_blink_rate: config.cursor_blink_rate(),
        };
        let now = Instant::now();
        AnimState {
            c,
            now,
            time_next_gc: now + TIMEOUT,
            text_cursor: Default::default(),
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
    pub fn text_cursor(&mut self, draw: &mut D, wid: u64, byte: usize) -> bool {
        match self.text_cursor.entry(wid) {
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
                entry.insert_entry(TextCursor { byte, state, time });
                draw.animate_at(time);
                true
            }
        }
    }
}
