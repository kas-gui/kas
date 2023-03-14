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
use kas::shell::{GraphicalShell, Result};

pub use draw_shaded::{DrawShaded, DrawShadedImpl};
pub use options::Options;
pub use shaded_theme::ShadedTheme;
pub extern crate wgpu;

/// Builder for a KAS shell using WGPU
pub struct WgpuShellBuilder<CB: CustomPipeBuilder>(CB, Options);

impl<CB: CustomPipeBuilder> GraphicalShell for WgpuShellBuilder<CB> {
    type Shared = DrawPipe<CB::Pipe>;
    type Window = draw::DrawWindow<<CB::Pipe as draw::CustomPipe>::Window>;
    type Surface = surface::Surface<CB::Pipe>;

    fn build(self) -> Result<Self::Shared> {
        DrawPipe::new(self.0, &self.1)
    }
}

impl Default for WgpuShellBuilder<()> {
    fn default() -> Self {
        WgpuShellBuilder((), Options::from_env())
    }
}

impl<CB: CustomPipeBuilder> From<CB> for WgpuShellBuilder<CB> {
    fn from(cb: CB) -> Self {
        WgpuShellBuilder(cb, Options::from_env())
    }
}

/// The default (unparameterised) implementation of [`WgpuShellBuilder`]
pub type DefaultGraphicalShell = WgpuShellBuilder<()>;
