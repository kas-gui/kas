// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS, the toolKit Abstraction Library
//!
//! KAS is a GUI library. This crate provides the following:
//!
//! -   a widget model: [`WidgetCore`], [`WidgetConfig`], [`Layout`],
//!     [`Widget`], [`event::Handler`] traits
//! -   a [`layout`] engine (mostly configured through [`macros`])
//! -   a modular [`draw`] API
//! -   widget [`event`] handling
//! -   some data types: [`geom`], [`Align`], [`Direction`]
//! -   some pre-build widgets: [`widget`] module
//!
//! See also these external crates:
//!
//! -   [`kas-theme`](https://crates.io/crates/kas-theme) - [docs.rs](https://docs.rs/crate/kas-theme) - theme API + themes
//! -   [`kas-wgpu`](https://crates.io/crates/kas-wgpu) - [docs.rs](https://docs.rs/crate/kas-wgpu) - WebGPU + winit integration

#![cfg_attr(feature = "nightly", feature(new_uninit))]

#[cfg(not(feature = "winit"))]
#[macro_use]
extern crate bitflags;

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
pub mod prelude;
pub mod widget;

// macro re-exports
pub mod macros;

// export most important members directly for convenience and less redundancy:
pub use crate::data::*;
pub use crate::toolkit::*;
pub use crate::traits::*;

/// Convenience definition: `Cow<'a, str>`
pub type CowStringL<'a> = std::borrow::Cow<'a, str>;

/// Convenience definition: `Cow<'static, str>`
pub type CowString = CowStringL<'static>;
