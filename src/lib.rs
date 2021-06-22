// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS GUI Toolkit
//!
//! KAS is a GUI library. This crate provides the following:
//!
//! -   the [`Widget`] trait family, with [`macros`] to implement them
//! -   a [`layout`] solver and [`event`] handling for widgets
//! -   building blocks including [`geom`] types and a [`draw`] API
//! -   some pre-build widgets: the [`widget`] module
//!
//! See also these external crates:
//!
//! -   [`kas-theme`](https://crates.io/crates/kas-theme) - [docs.rs](https://docs.rs/kas-theme) - theme API + themes
//! -   [`kas-wgpu`](https://crates.io/crates/kas-wgpu) - [docs.rs](https://docs.rs/kas-wgpu) - WebGPU + winit integration
//!
//! Also refer to:
//!
//! -   [KAS Tutorials](https://kas-gui.github.io/tutorials/)
//! -   [Examples](https://github.com/kas-gui/kas/tree/master/kas-wgpu/examples)
//! -   [Discuss](https://github.com/kas-gui/kas/discussions)

#![allow(clippy::or_fun_call, clippy::never_loop)]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "gat", feature(generic_associated_types))]
#![cfg_attr(feature = "min_spec", feature(min_specialization))]

#[macro_use]
extern crate bitflags;

pub extern crate easy_cast as cast;
extern crate kas_macros;
extern crate self as kas; // required for reliable self-reference in kas_macros

// internal modules:
mod core;
mod future;
mod toolkit;

// public implementations:
pub mod adapter;
pub mod class;
#[cfg(feature = "config")]
pub mod config;
pub mod dir;
pub mod draw;
pub mod event;
pub mod geom;
pub mod layout;
pub mod prelude;
pub mod text;
pub mod updatable;
pub mod widget;

// macro re-exports
pub mod macros;

// export most important members directly for convenience and less redundancy:
pub use crate::core::*;
pub use crate::future::*;
pub use crate::toolkit::*;
