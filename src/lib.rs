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
//! - [`easy-cast`](https://crates.io/crates/easy-cast) is re-export as [`cast`]
//! - `kas_macros` is an extended version of [`impl-tools`](https://crates.io/crates/impl-tools),
//!     re-export at the top-level
//! - [`kas_widgets`](https://crates.io/crates/kas-widgets) is re-export as [`widgets`](mod@widgets)
//! - [`kas_resvg`](https://crates.io/crates/kas-resvg) is re-export as [`resvg`] (`resvg` or `tiny-skia` feature)
//! - [`kas_view`](https://crates.io/crates/kas-view) is re-export as [`view`] (`view` feature)
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
    // - kas::Id appears to have no methods
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

pub mod app {
    //! Application, platforms and backends
    //!
    //! Start by constructing an [`Application`] or its [`Default`](type@Default)
    //! type-def (requires a backend be enabled, e.g. "wgpu").

    /// Application pre-launch state
    ///
    /// Suggested construction patterns:
    ///
    /// -   <code>kas::app::[Default](type@Default)::[new](Application::new)(data)?</code>
    /// -   <code>kas::app::[Default](type@Default)::[with_theme](Application::with_theme)(theme).[build](AppBuilder::build)(data)?</code>
    /// -   <code>kas::app::[WgpuBuilder]::[new](WgpuBuilder::new)(custom_wgpu_pipe).[with_theme](WgpuBuilder::with_theme)(theme).[build](AppBuilder::build)(data)?</code>
    ///
    /// Where:
    ///
    /// -   `data` is `()` or some object implementing [`AppData`]
    /// -   `theme` is some object implementing [`Theme`](crate::theme::Theme)
    /// -   `custom_wgpu_pipe` is a custom WGPU graphics pipeline
    #[doc(inline)]
    pub use kas_core::app::Application;

    pub use kas_core::app::*;

    #[cfg(feature = "wgpu")] pub use kas_wgpu::WgpuBuilder;

    /// Application pre-launch state, configured with the default graphics backend
    #[cfg(feature = "wgpu")]
    pub type Default<Data, T = crate::theme::FlatTheme> =
        Application<Data, kas_wgpu::WgpuBuilder<()>, T>;
}

#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use kas_dylib;
