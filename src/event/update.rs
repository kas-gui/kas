// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: updates

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};

/// An update handle
///
/// Update handles are used to trigger an update event on all widgets which are
/// subscribed to the same handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UpdateHandle(NonZeroU32);

impl UpdateHandle {
    /// Issue a new [`UpdateHandle`]
    ///
    /// A total of 2<sup>32</sup> - 1 update handles are available.
    /// Attempting to issue 2<sup>32</sup> handles will result in a panic.
    pub fn new() -> UpdateHandle {
        static COUNT: AtomicU32 = AtomicU32::new(0);

        loop {
            let c = COUNT.load(Ordering::Relaxed);
            let h = c.wrapping_add(1);
            let nz = NonZeroU32::new(h).unwrap_or_else(|| {
                panic!("UpdateHandle::new: all available handles have been issued")
            });
            if COUNT.compare_and_swap(c, h, Ordering::Relaxed) == c {
                break UpdateHandle(nz);
            }
        }
    }
}
