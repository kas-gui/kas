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
//! The [easy-cast](https://docs.rs/easy-cast/0.5/easy_cast) library is re-export as `kas_core::cast`.

// Use ``never_loop`` until: https://github.com/rust-lang/rust-clippy/issues/7397 is fixed
#![allow(clippy::identity_op, clippy::never_loop, clippy::enum_variant_names)]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "gat", feature(generic_associated_types))]
#![cfg_attr(feature = "spec", feature(specialization))]

extern crate self as kas;

#[macro_use]
extern crate bitflags;

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
