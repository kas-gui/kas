// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS shell over [winit] and [WGPU]
//!
//! This crate implements a KAS shell (backend) using [WGPU] for
//! GPU-accelerated rendering and [winit] for windowing, thus it should be
//! portable to most desktop and potentially also mobile platforms.
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
//! [winit]: https://github.com/rust-windowing/winit
//! [clipboard]: https://crates.io/crates/clipboard

#![cfg_attr(doc_cfg, feature(doc_cfg))]

pub mod draw;
mod draw_shaded;
mod event_loop;
pub mod options;
mod shaded_theme;
mod shared;
mod shell;
mod window;

use crate::draw::{CustomPipeBuilder, DrawPipe};
use kas::theme::RasterConfig;
use kas::WindowId;
use shell::ProxyAction;
use window::{Window, WindowSurface};

pub use draw_shaded::{DrawShaded, DrawShadedImpl};
pub use options::Options;
pub use shaded_theme::ShadedTheme;
pub use shell::{ClosedError, GraphicalShell, Proxy, Shell};
pub extern crate wgpu;
pub use kas::shell::*;

/// Builder for a KAS shell using WGPU
pub struct WgpuShellBuilder<CB: CustomPipeBuilder>(CB);

impl<CB: CustomPipeBuilder> GraphicalShell for WgpuShellBuilder<CB> {
    type Shared = DrawPipe<CB::Pipe>;
    type Surface = window::Surface<CB::Pipe>;

    fn build(self, options: &Options, raster_config: &RasterConfig) -> Result<Self::Shared> {
        DrawPipe::new(self.0, options, raster_config)
    }
}

impl Default for WgpuShellBuilder<()> {
    fn default() -> Self {
        WgpuShellBuilder(())
    }
}

impl<CB: CustomPipeBuilder> From<CB> for WgpuShellBuilder<CB> {
    fn from(cb: CB) -> Self {
        WgpuShellBuilder(cb)
    }
}

/// A KAS shell over Winit and WGPU
pub type Toolkit<C, T> = Shell<WgpuShellBuilder<C>, T>;
