// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS, the toolKit Abstraction Library
//!
//! KAS is a GUI library. This crate provides the following:
//!
//! -   a widget model: [`Widget`] trait
//! -   widget [`layout`] engine
//! -   widget [`event`] handling
//! -   common [`widget`] types
//!
//! The remaining functionality is provided by a separate crate, referred to as
//! the "toolkit". A KAS toolkit must provide:
//!
//! -   system interfaces (window creation and event capture)
//! -   widget rendering and sizing

#![cfg_attr(feature = "nightly", feature(new_uninit))]

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
pub mod widget;

// macro re-exports
pub mod macros;

// export most important members directly for convenience and less redundancy:
pub use crate::data::*;
pub use crate::toolkit::*;
pub use crate::traits::*;

#[cfg(feature = "stack_dst")]
/// Fixed-size object of `Unsized` type
///
/// This is a re-export of
/// [`stack_dst::ValueA`](https://docs.rs/stack_dst/0.6.0/stack_dst/struct.ValueA.html)
/// with a custom size. The `new` and `new_or_boxed` methods provide a
/// convenient API.
pub type StackDst<T> = stack_dst::ValueA<T, [usize; 8]>;
