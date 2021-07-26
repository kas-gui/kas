// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS theme support
//!
//! This crate allows widget rendering to be customised via themes,
//! and provides mid-level draw implementations for widgets.
//!
//! Each [`Theme`] is expected to have [`Window`]-specific data,
//! and provides an implementation of [`kas::draw::DrawHandle`].
//!
//! Two themes are provided by this trait: [`FlatTheme`] and [`ShadedTheme`].
//! Additionally, a meta-theme, [`MultiTheme`], allows run-time switching
//! between themes.

#![cfg_attr(feature = "gat", feature(generic_associated_types))]
#![cfg_attr(feature = "unsize", feature(unsize))]

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
pub use kas;

pub use colors::{Colors, ColorsLinear, ColorsSrgb};
pub use config::{Config, RasterConfig};
pub use draw_shaded::{DrawableShaded, DrawableShadedExt};
pub use flat_theme::FlatTheme;
#[cfg(feature = "stack_dst")]
pub use multi::{MultiTheme, MultiThemeBuilder};
pub use shaded_theme::ShadedTheme;
#[cfg(feature = "stack_dst")]
pub use theme_dst::{MaybeBoxed, ThemeDst, WindowDst};
pub use traits::{Theme, ThemeConfig, Window};

#[cfg(feature = "stack_dst")]
/// Fixed-size object of `Unsized` type
///
/// This is a re-export of
/// [`stack_dst::ValueA`](https://docs.rs/stack_dst/0.6.0/stack_dst/struct.ValueA.html)
/// with a custom size. The `new` and `new_or_boxed` methods provide a
/// convenient API.
///
/// **Feature gated**: this is only available with feature `stack_dst`.
pub type StackDst<T> = stack_dst_::ValueA<T, [usize; 9]>;
