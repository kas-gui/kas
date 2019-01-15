// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Callback specific code
//! 
//! Note that callbacks are added to windows, hence some callback functionality
//! is a detail of the [`Window`] trait.

/// A Condition specifies when a callback is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// Call once immediately on start
    // Note: we call this when the event loop starts. We could perhaps use
    // GDK's Map event instead, which is called when the window is created and
    // when it is restored from a minimised state.
    Start,
    /// Call repeatedly after a timeout specified in milliseconds. Precise
    /// timing is not guaranteed.
    // Note: do we want to auto-suspend timeouts for minimised windows? Perhaps
    // make this optional?
    TimeoutMs(u32),
    /// Call repeatedly after a timeout specified in seconds. Precise timing is
    /// not guaranteed.
    TimeoutSec(u32),
}
