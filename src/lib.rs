// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS, the toolKit Abstraction Library
//! 
//! The KAS library is designed for concise, programatic descriptions of GUIs
//! which are translated to some backend "toolkit" on use.
#![feature(unrestricted_attribute_tokens)]
#![feature(extern_crate_self)]

#[doc(hidden)]
#[cfg(feature = "cassowary")]
pub extern crate cassowary as cw;    // used by macros

extern crate self as kas; // required for reliable self-reference in kas_macros
extern crate kas_macros;

// internal modules:
#[macro_use]
mod widget;
mod window;
mod toolkit;
mod traits;

// public implementations:
pub mod callback;
pub mod control;
pub mod dialog;
pub mod text;
pub mod event;

// macro re-exports
pub mod macros;

// export most important members directly for convenience and less redundancy:
pub use crate::widget::*;
pub use crate::window::*;
pub use crate::toolkit::*;
pub use crate::traits::*;
