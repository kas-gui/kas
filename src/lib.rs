// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS GUI Toolkit
//!
//! This is the main KAS crate, featuring:
//!
//! -   the [`Widget`] trait family, with [`macros`] to implement them
//! -   high-level themable and mid-level [`draw`] APIs
//! -   [`event`] handling code
//! -   [`geom`]-etry types and widget [`layout`] solvers
//! -   a [`widget`] library
//!
//! See also these external crates:
//!
//! -   `kas-theme` - [crates.io](https://crates.io/crates/kas-theme) - [docs.rs](https://docs.rs/kas-theme) - theme API + themes
//! -   `kas-wgpu` - [crates.io](https://crates.io/crates/kas-wgpu) - [docs.rs](https://docs.rs/kas-wgpu) - WebGPU + winit integration
//!
//! Also refer to:
//!
//! -   [KAS Tutorials](https://kas-gui.github.io/tutorials/)
//! -   [Examples](https://github.com/kas-gui/kas/tree/master/kas-wgpu/examples)
//! -   [Discuss](https://github.com/kas-gui/kas/discussions)

// Use ``never_loop`` until: https://github.com/rust-lang/rust-clippy/issues/7397 is fixed
#![allow(
    clippy::identity_op,
    clippy::or_fun_call,
    clippy::never_loop,
    clippy::comparison_chain
)]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "min_spec", feature(min_specialization))]

extern crate kas_macros;
extern crate self as kas; // required for reliable self-reference in kas_macros

// public implementations:
pub mod adapter;
pub mod prelude;
pub mod widget;

// macro re-exports
pub mod macros;

pub use kas_core::*;
