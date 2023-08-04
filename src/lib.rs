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
//! - [`easy-cast`](https://docs.rs/easy-cast/0.5) is re-export as [`kas::cast`](cast)
//! - `kas_macros` is an extended version of [`impl-tools`](https://docs.rs/impl-tools/),
//!     re-export at the top-level
//! - [`kas_widgets`](https://docs.rs/kas-widgets/0.11) is re-export as [`kas::widgets`](mod@widgets)
//! - [`kas_resvg`] is re-export as [`kas::resvg`](resvg) (`resvg` or `tiny-skia` feature)
//! - [`kas_view`](https://docs.rs/kas-view/0.11) is re-export as [`kas::view`](view) (`view` feature)
//! - [`kas_wgpu`](https://docs.rs/kas-wgpu/0.11) is re-export as [`kas::shell`](shell); in the current version
//!     this is dependant on [WGPU](https://github.com/gfx-rs/wgpu), but in the
//!     future this should become a shim over multiple back-ends
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

    #[doc(no_inline)] pub use kas_core::prelude::*;
    #[doc(no_inline)]
    pub use kas_widgets::adapt::{AdaptWidget, AdaptWidgetAny};
}

pub use kas_core::*;

#[doc(inline)] pub extern crate kas_widgets as widgets;

#[cfg(feature = "view")]
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

pub mod shell {
    //! Shell: window runtime environment
    //!
    //! A [`Shell`] is used to manage a GUI. Most GUIs will use the
    //! [`DefaultShell`] type-def (requires a backend be enabled, e.g. "wgpu").

    pub use kas_core::shell::*;

    /// The WGPU shell
    #[cfg(feature = "wgpu")]
    pub type WgpuShell<Data, CB, T> =
        kas_core::shell::Shell<Data, kas_wgpu::WgpuShellBuilder<CB>, T>;

    /// The default (configuration-specific) shell
    #[cfg(feature = "wgpu")]
    pub type DefaultShell<Data, T> =
        kas_core::shell::Shell<Data, kas_wgpu::DefaultGraphicalShell, T>;
}

#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use kas_dylib;
