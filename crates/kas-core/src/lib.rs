// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS GUI core
//!
//! Re-exports:
//!
//! -   [`kas::cast`] is a re-export of [`easy-cast`](https://crates.io/crates/easy-cast)
//! -   [`impl_self`], [`impl_scope!`], [`impl_anon!`], [`autoimpl`] and [`impl_default`] are
//!     re-implementations of [`impl-tools`](https://crates.io/crates/impl-tools) macros

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "spec", feature(specialization))]

extern crate self as kas;

#[macro_use] extern crate bitflags;

#[doc(inline)] pub extern crate easy_cast as cast;

// internal modules:
#[cfg(feature = "accesskit")] pub(crate) mod accesskit;
mod action;
mod core;
pub mod widgets;
pub mod window;

pub use crate::core::*;
pub use action::Action;
pub use kas_macros::{autoimpl, extends, impl_default};
pub use kas_macros::{cell_collection, collection, impl_anon, impl_scope, impl_self};
pub use kas_macros::{layout, widget, widget_index, widget_set_rect};

// public implementations:
pub mod config;
pub mod dir;
pub mod draw;
pub mod event;
pub mod geom;
pub mod layout;
pub mod messages;
pub mod prelude;
pub mod runner;
pub mod text;
pub mod theme;
pub mod util;
