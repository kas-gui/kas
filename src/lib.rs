// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS GUI Toolkit
//!
//! This, the main KAS crate, is a wrapper over other crates designed to make
//! content easily available while remaining configurable. The following crates
//! (some optional, dependant on a feature flag) are re-exported by this crate:
//!
//! - [`kas_core`] is re-export at the top-level
//! - [`easy-cast`](https://crates.io/crates/easy-cast) is re-export as [`kas::cast`](cast)
//! - `kas_macros` is an extended version of [`impl-tools`](https://crates.io/crates/impl-tools),
//!     re-export as [`kas::macros`](macros)
//! - [`kas_widgets`] is re-export as [`kas::widgets`](widgets)
//! - [`kas_resvg`] is re-export as [`kas::resvg`](resvg) (`resvg` or `tiny-skia` feature)
//! - [`kas_view`] is re-export as [`kas::view`](view) (`view` feature)
//! - [`kas_theme`] (`theme` feature) is re-export under
//!     [`kas::theme`](kas_theme); note that this module contains content from
//!     both [`kas_theme`] and [`kas_core::theme`]
//! - [`kas_wgpu`] is re-export as [`kas::shell`](shell); in the current version
//!     this is dependant on [WGPU](https://github.com/gfx-rs/wgpu), but in the
//!     future this should become a shim over multiple back-ends
//! - [`kas_dylib`] (`dynamic` feature) is used dynamic linking; this crate is
//!     not used directly — simply enabling the `dynamic` feature is enough to
//!     use dynamic linking.
//!
//! Also refer to:
//!
//! -   [KAS Tutorials](https://kas-gui.github.io/tutorials/)
//! -   [Examples](https://github.com/kas-gui/kas/tree/master/examples)
//! -   [Discuss](https://github.com/kas-gui/kas/discussions)

#![cfg_attr(doc_cfg, feature(doc_cfg))]

/// KAS prelude
///
/// This module allows convenient importation of common unabiguous items:
/// ```
/// use kas::prelude::*;
/// ```
///
/// This prelude may be more useful when implementing widgets than when simply
/// using widgets in a GUI.
pub mod prelude {
    // Note: using #[doc(no_inline)] here causes doc issues in this crate:
    // - kas::WidgetId appears to have no methods
    // - doc_cfg annotations appear to be attached to the wrong items

    pub use kas_core::prelude::*;
    pub use kas_widgets::adapter::AdaptWidget;
}

pub use kas_core::*;

#[doc(inline)]
pub extern crate kas_widgets as widgets;

#[cfg(any(feature = "view"))]
#[cfg_attr(doc_cfg, doc(cfg(feature = "view")))]
#[doc(inline)]
pub extern crate kas_view as view;

/// `Canvas` and `Svg` widgets over [`tiny-skia`](https://crates.io/crates/tiny-skia)
/// and [`resvg`](https://crates.io/crates/resvg)
///
/// This crate provides widgets using
/// libraries by [Yevhenii Reizner "RazrFalcon"](https://github.com/RazrFalcon/).
///
/// This module is gated behind the `resvg` feature. Alternatively, the
/// `tiny-skia` feature may be used to enable only the `Canvas` widget
/// plus support (i.e. everything but `Svg`), saving approx 200 KiB.
#[cfg(any(feature = "resvg", feature = "tiny-skia"))]
#[cfg_attr(doc_cfg, doc(cfg(feature = "resvg")))]
pub mod resvg {
    pub use kas_resvg::*;
}

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
#[doc(inline)]
pub extern crate kas_wgpu as shell;

#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use kas_dylib;
