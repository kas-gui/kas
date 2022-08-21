// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS GUI Toolkit
//!
//! This, the main KAS crate, is a wrapper over other crates designed to make
//! content easily available while remaining configurable.
//! Since generated documentation for re-exported items is poor, the crates used
//! are listed below:
//!
//! **Crate [`easy-cast`](https://crates.io/crates/easy-cast):** `Conv`, `Cast` traits and related functionality
//! (always included), available as [`kas::cast`](cast).
//!
//! **Crate [`kas_core`]:** this is the core crate (always included).
//! Its contents are re-exported directly from the root of this crate.
//!
//! **Crate `kas_macros`:** procedural macros (always included), available
//! as [`kas::macros`](kas_core::macros).
//!
//! **Crate [`kas_widgets`]:** common widget implementations (always included).
//! These are available as [`kas::widgets`](kas_widgets).
//!
//! **Crate [`kas_resvg`]:** `Canvas` and `Svg` widgets over crate
//! [resvg](https://github.com/RazrFalcon/resvg) and associated libraries.
//! Gated under the feature `resvg` (enabled by default) or `tiny-skia` and
//! available as [`kas::resvg`](resvg).
//!
//! **Crate [`kas_view`]:** "view" widgets over shared data
//! (gated under feature `view`, enabled by default).
//! These are available as [`kas::view`](kas_view).
//!
//! **Crate [`kas_theme`]:** switchable theme support and high-level drawing
//! (gated under feature `theme`, enabled by default, and hard to do without).
//! This is available as [`kas::theme`](kas_theme).
//!
//! **Crate [`kas_wgpu`]:** the shell, providing system integration and graphics
//! implementations over [WGPU](https://github.com/gfx-rs/wgpu).
//! This crate is gated under feature `wgpu` (enabled by default),
//! but until an alternative is available it is essential.
//! Its contents are available as [`kas::shell`](kas_wgpu).
//!
//! **Crate [`kas_dylib`]:** a support crate for dynamic linking (gated under
//! the feature `dynamic`). Its contents should not be used directly; simply
//! enabling the `dynamic` feature is enough to use dynamic linking.
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
