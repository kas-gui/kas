// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS theme support
//!
//! This crate provides the [`Theme`] trait, [`MultiTheme`] adapter, color
//! schemes, some supporting items, and the themes [`FlatTheme`] and
//! [`ShadedTheme`].
//!
//! Custom themes may be built over this crate, optionally including custom draw
//! routines (e.g. [`DrawShaded`]), provided that the shell implements support.
//! Alternatively this crate may be skipped altogether, especially for a
//! minimal shell with a custom fixed theme.

#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "gat", feature(generic_associated_types))]
#![cfg_attr(feature = "unsize", feature(unsize))]

mod anim;
mod colors;
mod config;
mod draw_shaded;
mod flat_theme;
#[cfg(feature = "stack_dst")]
mod multi;
mod shaded_theme;
#[cfg(feature = "stack_dst")]
mod theme_dst;
mod traits;

pub mod dim;

pub use colors::{Colors, ColorsLinear, ColorsSrgb};
pub use config::{Config, RasterConfig};
pub use draw_shaded::{DrawShaded, DrawShadedImpl};
pub use flat_theme::FlatTheme;
#[cfg(feature = "stack_dst")]
pub use multi::{MultiTheme, MultiThemeBuilder};
pub use shaded_theme::ShadedTheme;
#[cfg(feature = "stack_dst")]
pub use theme_dst::{MaybeBoxed, ThemeDst};
pub use traits::{Theme, ThemeConfig, Window};

#[cfg(feature = "stack_dst")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "stack_dst")))]
/// Fixed-size object of `Unsized` type
///
/// This is a re-export of
/// [`stack_dst::ValueA`](https://docs.rs/stack_dst/0.6.0/stack_dst/struct.ValueA.html)
/// with a custom size. The `new` and `new_or_boxed` methods provide a
/// convenient API.
///
/// **Feature gated**: this is only available with feature `stack_dst`.
pub type StackDst<T> = stack_dst_::ValueA<T, [usize; 9]>;
