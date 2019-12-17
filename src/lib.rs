// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#![cfg_attr(feature = "nightly", feature(new_uninit))]

//! KAS, the toolKit Abstraction Library
//!
//! KAS is a GUI library, providing approximately two thirds of GUI toolkit
//! functionality:
//!
//! -   widget model
//! -   widget layout engine
//! -   widget event handling
//! -   widget types
//!
//! The remaining third is provided by a separate crate, referred to as the
//! "toolkit". A KAS toolkit must provide:
//!
//! -   system interfaces (window creation and event capture)
//! -   widget rendering and sizing

extern crate kas_macros;
extern crate self as kas; // required for reliable self-reference in kas_macros

// internal modules:
mod data;
mod toolkit;
mod traits;

// public implementations:
pub mod class;
pub mod draw;
pub mod event;
pub mod geom;
pub mod layout;
pub mod theme;
pub mod widget;

// macro re-exports
pub mod macros;

// export most important members directly for convenience and less redundancy:
pub use crate::data::*;
pub use crate::toolkit::*;
pub use crate::traits::*;

use proc_macro_hack::proc_macro_hack;

/// See [The `make_widget` macro]: #the-make_widget-macro
#[proc_macro_hack(support_nested)]
pub use kas_macros::make_widget;
