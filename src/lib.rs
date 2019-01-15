// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS lib
#![feature(unrestricted_attribute_tokens)]

#[doc(hidden)]
#[cfg(feature = "cassowary")]
pub extern crate cassowary as cw;    // used by macros

extern crate kas_macros;

// internal modules:
#[macro_use]
mod widget;
mod window;
mod toolkit;

// public implementations:
pub mod callback;
pub mod control;
pub mod dialog;
pub mod display;
pub mod event;

/// Library macros
/// 
/// Note that some of these are re-exports, but it is expected that users depend on this crate only
/// and not `kas_macros`. All functionality is available via these re-exports.
pub mod macros {
    pub use kas_macros::{NoResponse, Widget, make_widget};
}

// export most important members directly for convenience and less redundancy:
pub use crate::widget::*;
pub use crate::window::*;
pub use crate::toolkit::*;
