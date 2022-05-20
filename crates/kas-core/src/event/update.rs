// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: updates

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// An update identifier
///
/// Used to identify the origin of an [`Event::Update`](crate::event::Event::Update).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[must_use]
pub struct UpdateId(NonZeroU32);

impl UpdateId {
    /// Issue a new [`UpdateId`]
    ///
    /// A total of 2<sup>32</sup> - 1 update handles are available.
    /// Attempting to issue 2<sup>32</sup> handles will result in a panic.
    pub fn new() -> UpdateId {
        static COUNT: AtomicU32 = AtomicU32::new(0);

        loop {
            let c = COUNT.load(Relaxed);
            let h = c.wrapping_add(1);
            let nz = NonZeroU32::new(h)
                .unwrap_or_else(|| panic!("UpdateId::new: all available handles have been issued"));
            if COUNT.compare_exchange(c, h, Relaxed, Relaxed).is_ok() {
                break UpdateId(nz);
            }
        }
    }
}
impl Default for UpdateId {
    fn default() -> Self {
        Self::new()
    }
}
