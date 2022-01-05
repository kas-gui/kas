// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS GUI core

// Use ``never_loop`` until: https://github.com/rust-lang/rust-clippy/issues/7397 is fixed
#![allow(clippy::identity_op, clippy::never_loop)]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "gat", feature(generic_associated_types))]
#![cfg_attr(feature = "spec", feature(specialization))]

#[macro_use]
extern crate bitflags;

pub extern crate easy_cast as cast;
pub extern crate kas_macros as macros;

// internal modules:
mod core;
mod future;
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
pub mod prelude;
pub mod text;
pub mod theme;
pub mod updatable;
pub mod util;

// export most important members directly for convenience and less redundancy:
pub use crate::core::*;
pub use crate::future::*;
pub use crate::toolkit::*;
