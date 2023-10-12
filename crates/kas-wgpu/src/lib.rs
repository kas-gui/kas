// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS shell over [WGPU]
//!
//! This crate implements a KAS's drawing APIs over [WGPU].
//!
//! This crate supports themes via the [`kas::theme`], and provides one
//! additional theme, [`ShadedTheme`].
//!
//! Custom GPU-accelerated drawing is supported via [`draw::CustomPipe`]
//! (see the [Mandlebrot example](https://github.com/kas-gui/kas/blob/master/kas-wgpu/examples/mandlebrot.rs)).
//!
//! By default, some environment variables are read for configuration.
//! See [`options::Options::from_env`] for documentation.
//!
//! [WGPU]: https://github.com/gfx-rs/wgpu

#![cfg_attr(doc_cfg, feature(doc_cfg))]

pub mod draw;
mod draw_shaded;
pub mod options;
mod shaded_theme;
mod surface;

use crate::draw::{CustomPipeBuilder, DrawPipe};
use kas::shell::{GraphicalShell, Result, ShellBuilder};
use kas::theme::{FlatTheme, Theme};

pub use draw_shaded::{DrawShaded, DrawShadedImpl};
pub use options::Options;
pub use shaded_theme::ShadedTheme;
pub extern crate wgpu;

/// Builder for a KAS shell using WGPU
pub struct WgpuBuilder<CB: CustomPipeBuilder>(CB, Options);

impl<CB: CustomPipeBuilder> GraphicalShell for WgpuBuilder<CB> {
    type DefaultTheme = FlatTheme;

    type Shared = DrawPipe<CB::Pipe>;

    type Surface = surface::Surface<CB::Pipe>;

    fn build(self) -> Result<Self::Shared> {
        DrawPipe::new(self.0, &self.1)
    }
}

impl Default for WgpuBuilder<()> {
    fn default() -> Self {
        WgpuBuilder::new(())
    }
}

impl<CB: CustomPipeBuilder> WgpuBuilder<CB> {
    /// Construct with the given pipe builder
    ///
    /// Pass `()` or use [`Self::default`] when not using a custom pipe.
    pub fn new(cb: CB) -> Self {
        WgpuBuilder(cb, Options::from_env())
    }

    /// Convert to a [`ShellBuilder`] using the default theme
    #[inline]
    pub fn with_default_theme(self) -> ShellBuilder<Self, FlatTheme> {
        ShellBuilder::new(self, FlatTheme::new())
    }

    /// Convert to a [`ShellBuilder`] using the specified `theme`
    #[inline]
    pub fn with_theme<T: Theme<DrawPipe<CB::Pipe>>>(self, theme: T) -> ShellBuilder<Self, T> {
        ShellBuilder::new(self, theme)
    }
}
