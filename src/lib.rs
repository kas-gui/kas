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
//!   re-export at the top-level
//! - [`kas_widgets`](https://crates.io/crates/kas-widgets) is re-export as [`widgets`](mod@widgets)
//! - [`kas_image`](https://crates.io/crates/kas-image) is re-export as [`image`] (`svg` or `canvas` feature)
//! - [`kas_view`](https://crates.io/crates/kas-view) is re-export as [`view`] (`view` feature)
//!
//! Also refer to:
//!
//! -   [KAS Tutorials](https://kas-gui.github.io/tutorials/)
//! -   [Examples](https://github.com/kas-gui/kas/tree/master/examples)
//! -   [Discuss](https://github.com/kas-gui/kas/discussions)

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

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
    #[doc(no_inline)] pub use kas_core::prelude::*;
    #[doc(no_inline)]
    pub use kas_widgets::adapt::{AdaptWidget, AdaptWidgetAny};
}

pub use kas_core::*;

#[doc(inline)] pub extern crate kas_widgets as widgets;

#[cfg(feature = "view")]
#[doc(inline)]
pub extern crate kas_view as view;

/// `Canvas` and `Svg` widgets over [`tiny-skia`](https://crates.io/crates/tiny-skia)
/// and [`resvg`](https://crates.io/crates/resvg)
///
/// This crate provides widgets using
/// libraries by [Yevhenii Reizner "RazrFalcon"](https://github.com/RazrFalcon/).
#[cfg(any(feature = "svg", feature = "canvas"))]
pub mod image {
    pub use kas_image::*;
}

pub mod runner;

#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use kas_dylib;
