// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Callback specific code
//!
//! Note that callbacks are added to windows, hence some callback functionality
//! is a detail of the [`Window`] trait.
//!
//! [`Window`]: crate::Window

use std::time::Duration;

/// Specifies under which condition a callback is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callback {
    /// Call once immediately on start.
    Start,
    /// Call on start and repeatedly with the given period. Precise timing is not guaranteed.
    // Note: do we want to auto-suspend timeouts for minimised windows? Perhaps
    // make this optional?
    Repeat(Duration),
}
