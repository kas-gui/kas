// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS GUI core
//!
//! Re-exports:
//!
//! -   [`kas::cast`] is a re-export of [`easy-cast`](https://crates.io/crates/easy-cast)
//! -   [`impl_scope!`], [`singleton!`], [`autoimpl`] and [`impl_default`] are
//!     re-implementations of [`impl-tools`](https://crates.io/crates/impl-tools) macros

#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "spec", feature(specialization))]

extern crate self as kas;

#[macro_use] extern crate bitflags;

#[doc(inline)] pub extern crate easy_cast as cast;

// internal modules:
mod action;
mod core;
mod decorations;
mod erased;
mod root;

pub use crate::core::*;
pub use action::Action;
pub use decorations::Decorations;
pub use erased::{AppData, Erased, ErasedStack};
pub use kas_macros::*;
#[doc(inline)]
pub use root::{Window, WindowCommand, WindowId};

// public implementations:
pub mod class;
pub mod config;
pub mod dir;
pub mod draw;
pub mod event;
pub mod geom;
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub mod hidden;
pub mod layout;
pub mod message;
pub mod prelude;
pub mod shell;
pub mod text;
pub mod theme;
pub mod util;
