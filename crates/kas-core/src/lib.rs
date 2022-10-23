// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS GUI core
//!
//! This core library provides:
//!
//! -   the [`Widget`] trait family, with [`macros`] to implement them
//! -   high-level themable and mid-level [`draw`] APIs
//! -   [`event`] handling code
//! -   [`geom`]-etry types and widget [`layout`] solvers
//!
//! **Crate [`easy-cast`](https://crates.io/crates/easy-cast):** `Conv`, `Cast` traits and related functionality
//! (always included), available as [`kas::cast`](https://docs.rs/easy-cast/0.5/easy_cast).

#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "spec", feature(specialization))]

extern crate self as kas;

#[macro_use] extern crate bitflags;

pub extern crate easy_cast as cast;
pub extern crate kas_macros as macros;

// internal modules:
mod core;
mod root;
mod toolkit;

// public implementations:
pub mod class;
#[cfg(feature = "config")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "config")))]
pub mod config;
pub mod dir;
pub mod draw;
pub mod event;
pub mod geom;
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub mod label;
pub mod layout;
pub mod model;
pub mod prelude;
pub mod text;
pub mod theme;
pub mod util;

// export most important members directly for convenience and less redundancy:
pub use crate::core::*;
pub use crate::toolkit::*;
pub use root::RootWidget;
