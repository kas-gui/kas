// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! # Draw APIs
//!
//! Multiple drawing APIs are available. Each has a slightly different purpose:
//!
//! -   High-level "themed widget components" are available through
//!     [`DrawCx`]. This is the primary drawing interface for widgets.
//! -   Basic drawing components (shapes) are available through [`DrawIface`]
//!     in this module. This can be accessed via [`DrawCx::draw_device`].
//! -   The shell may support custom graphics pipelines, for example
//!     [`kas-wgpu::draw::CustomPipe`](https://docs.rs/kas-wgpu/*/kas_wgpu/draw/trait.CustomPipe.html)
//!     (used by the [Mandlebrot example](https://github.com/kas-gui/kas/tree/master/examples/mandlebrot)).
//!
//! Text may be drawn by either [`DrawCx`] or [`DrawIface`] with a slightly
//! different API (using theme properties or directly specifying colors and effects).
//!
//! ## Draw order
//!
//! All draw operations happen within a "draw pass". The first pass corresponds
//! to the window, while additional passes may be clipped and offset (see
//! [`DrawIface::new_pass`]). Draw passes are executed sequentially in the order
//! defined.
//!
//! Within each pass, draw operations may be batched by the shell, thus draw
//! operations may not happen in the order queued. In general, it may be
//! expected that batches are executed in the following order:
//!
//! 1.  Square-edged primitives (e.g. [`Draw::rect`])
//! 2.  Images
//! 3.  Rounded or other partially-transparent primitives (e.g. [`DrawRounded::circle`])
//! 4.  Custom draw routines (`CustomPipe`)
//! 5.  Text

pub mod color;

#[allow(clippy::module_inception)] mod draw;
mod draw_rounded;
mod draw_shared;

use crate::cast::Cast;
#[allow(unused)] use crate::theme::DrawCx;

pub use draw::{Draw, DrawIface, DrawImpl};
pub use draw_rounded::{DrawRounded, DrawRoundedImpl};
pub use draw_shared::{AllocError, ImageFormat, ImageHandle, ImageId};
pub use draw_shared::{DrawShared, DrawSharedImpl, SharedState};
use std::time::{Duration, Instant};

/// Animation status
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[crate::impl_default(AnimationState::None)]
pub(crate) enum AnimationState {
    /// No frames are queued
    None,
    /// Animation in progress: draw at the next frame time
    Animate,
    /// Timed-animation in progress: draw at the given time
    Timed(Instant),
}

impl AnimationState {
    /// Merge two states (take earliest)
    fn merge_in(&mut self, rhs: AnimationState) {
        use AnimationState::*;
        *self = match (*self, rhs) {
            (Animate, _) | (_, Animate) => Animate,
            (Timed(t1), Timed(t2)) => Timed(t1.min(t2)),
            (Timed(t), _) | (_, Timed(t)) => Timed(t),
            (None, None) => None,
        }
    }
}

/// Per-window "draw" data common to all backends
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[derive(Debug, Default)]
pub struct WindowCommon {
    pub(crate) anim: AnimationState,
    pub(crate) dur_text: std::time::Duration,
}

impl WindowCommon {
    /// Report performance counter: text assembly duration
    ///
    /// This may be reported multiple times per frame; the sum is output.
    pub fn report_dur_text(&mut self, dur: Duration) {
        self.dur_text += dur;
    }
}

/// Draw pass identifier
///
/// This is a numerical identifier for the draw pass (see [`DrawIface::new_pass`]).
#[derive(Copy, Clone)]
pub struct PassId(u32);

impl PassId {
    /// Construct a new pass from a `u32` identifier
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[inline]
    pub const fn new(n: u32) -> Self {
        PassId(n)
    }

    /// The pass number
    ///
    /// This value is returned as `usize` but is always safe to store `as u32`.
    #[inline]
    pub fn pass(self) -> usize {
        self.0.cast()
    }

    /// The depth value
    ///
    /// This is a historical left-over and always returns 0.0.
    #[inline]
    pub fn depth(self) -> f32 {
        0.0
    }
}

/// Type of draw pass
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum PassType {
    /// New pass is clipped and offset relative to parent
    Clip,
    /// New pass is an overlay
    ///
    /// An overlay is a layer drawn over the base window, for example a tooltip
    /// or combobox menu. The rect and offset are relative to the base window.
    /// The theme may draw a shadow or border around this rect.
    Overlay,
}
