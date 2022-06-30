// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS GUI Toolkit
//!
//! This, the main KAS crate, is merely a wrapper over other crates:
//!
//! -   [`kas_core`] is the core of the library
//! -   [`kas_theme`] provides high-level (themed) drawing
//! -   [`kas_widgets`] provides common high-level widgets
//! -   [`kas_wgpu`] is the shell, providing system integration and graphics
//!     implementations (this should become less dependant on WGPU in the future)
//!
//! All items from [`kas_core`] are directly re-exported from this crate
//! (e.g. [`kas::geom::Size`](crate::geom::Size)); other crates are re-exported
//! as a sub-module (e.g. [`kas::shell::Toolkit`](crate::shell::Toolkit)).
//!
//! The [easy-cast](https://docs.rs/easy-cast/0.5/easy_cast) library is re-export as `kas::cast`.
//!
//! Also refer to:
//!
//! -   [KAS Tutorials](https://kas-gui.github.io/tutorials/)
//! -   [Examples](https://github.com/kas-gui/kas/tree/master/examples)
//! -   [Discuss](https://github.com/kas-gui/kas/discussions)
//! -   [easy-cast API docs](https://docs.rs/easy-cast) (this is re-exported as `cast`)

#![cfg_attr(doc_cfg, feature(doc_cfg))]

// public implementations:
pub mod prelude;

pub use kas_core::*;

pub extern crate kas_widgets as widgets;

#[cfg(any(feature = "canvas", feature = "svg"))]
#[cfg_attr(doc_cfg, doc(cfg(any(feature = "canvas", feature = "svg"))))]
pub extern crate kas_resvg as resvg;

/// Themes
///
/// This module merges [`kas_core::theme`](https://docs.rs/kas-theme/0.11/kas_theme) and [`kas_theme`].
pub mod theme {
    pub use kas_core::theme::*;

    #[cfg(feature = "theme")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "theme")))]
    pub use kas_theme::*;
}

#[cfg(feature = "wgpu")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "wgpu")))]
pub extern crate kas_wgpu as shell;

#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use kas_dylib;
