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
//! -   the standard [`widgets`] library
//!
//! See also these external crates:
//!
//! -   `kas-theme` - [crates.io](https://crates.io/crates/kas-theme) - [docs.rs](https://docs.rs/kas-theme) - theme API + themes
//! -   `kas-wgpu` - [crates.io](https://crates.io/crates/kas-wgpu) - [docs.rs](https://docs.rs/kas-wgpu) - wgpu + winit backend
//!
//! Also refer to:
//!
//! -   [KAS Tutorials](https://kas-gui.github.io/tutorials/)
//! -   [Examples](https://github.com/kas-gui/kas/tree/master/kas-wgpu/examples)
//! -   [Discuss](https://github.com/kas-gui/kas/discussions)
//! -   [easy-cast API docs](https://docs.rs/easy-cast) (this is re-exported as `cast`)

#![cfg_attr(doc_cfg, feature(doc_cfg))]

// public implementations:
pub mod prelude;

pub use kas_core::*;

pub use kas_widgets as widgets;

#[cfg(any(feature = "canvas", feature = "svg"))]
pub use kas_resvg as resvg;

/// Themes
///
/// This module merges [`kas_core::theme`] and (with the `theme` feature) [`kas_theme`].
pub mod theme {
    pub use kas_core::theme::*;

    #[cfg(feature = "theme")]
    pub use kas_theme::*;
}

#[cfg(feature = "wgpu")]
pub use kas_wgpu as shell;

#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use kas_dylib;
