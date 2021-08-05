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
//! -   [easy-cast API docs](https://docs.rs/easy-cast) (this is re-exported as `cast`)

#![cfg_attr(doc_cfg, feature(doc_cfg))]

// public implementations:
pub mod prelude;

// macro re-exports
pub mod macros;

// include most of kas_core, excluding macros and prelude:
#[cfg(feature = "config")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "config")))]
pub use kas_core::config;
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub use kas_core::ShellWindow;
pub use kas_core::{cast, class, dir, draw, event, geom, layout, text, updatable, util};
pub use kas_core::{Boxed, Layout, LayoutData, Window};
pub use kas_core::{CoreData, Future, Popup, TkAction, WidgetId, WindowId};
pub use kas_core::{Widget, WidgetChildren, WidgetConfig, WidgetCore};

pub use kas_widgets as widget;

#[cfg(feature = "theme")]
pub use kas_theme as theme;

#[cfg(feature = "wgpu")]
pub use kas_wgpu as shell;

#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use kas_dylib;
