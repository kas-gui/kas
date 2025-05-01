// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: mouse / touch velocity sampling
//!
//! The goal here is to yield estimates of velocity (in pixels per second) for
//! mouse motion and touch swipe events. Estimates should be smooth (ideally at
//! least two delta samples) and responsive (ideally no longer than the period
//! of one frame).
//!
//! From a cursory web search and some tests on available devices:
//!
//! - Common (basic) mice usually report at 125 Hz while gaming mice commonly
//!   report at 1 kHz (potentially up to 8 kHz)
//! - Some touchscreens report at ~100 Hz while modern phone touchscreens often
//!   report at twice the screen refresh rate (with some gaming phones using
//!   ~1-2 kHz).
//! - A Windows Precision Touchpad is required to report at 125 Hz; my Elan
//!   touchpad reports at 150 Hz.
//! - Troubleshooting requests attest that rates are sometimes much lower (as
//!   low as 30 Hz).
//!
//! A sample period of `3500 / screen_refresh_hz` ms should allow at least 3
//! (delta) samples while providing 7 in the common 125 Hz mouse, 60 Hz screen
//! case. Provided we ignore deltas of zero, we may limit our sample buffer to
//! a relatively small size (e.g. 8 samples) without making results too jittery;
//! this also improves responsiveness when the sample rate is high.

use crate::geom::Vec2;
use smallvec::SmallVec;
use std::time::{Duration, Instant};

const MAX_SAMPLES: usize = 8;

/// A buffer of recent delta samples used to estimate velocity
#[derive(Clone, Debug, Default)]
pub(super) struct Samples {
    samples: SmallVec<[(Instant, Vec2); MAX_SAMPLES]>,
    next: usize, // index of next insert
}

impl Samples {
    /// Clear all samples
    pub(super) fn clear(&mut self) {
        self.samples.clear();
        self.next = 0;
    }

    /// Push a new sample
    pub(super) fn push_delta(&mut self, delta: Vec2) {
        let now = Instant::now();
        if self.samples.len() < MAX_SAMPLES {
            self.samples.push((now, delta));
        } else {
            self.samples[self.next] = (now, delta);
            self.next = (self.next + 1) % MAX_SAMPLES;
        }
    }

    /// Calculate average velocity over a given sample `period`
    ///
    /// Units: pixels per second.
    pub(super) fn velocity(&self, period: Duration) -> Vec2 {
        let now = Instant::now();
        let start = now - period; // saturating_sub

        let mut delta = Vec2::ZERO;
        for sample in &self.samples {
            if sample.0 > start {
                delta += sample.1;
            }
        }

        delta / period.as_secs_f32()
    }
}
