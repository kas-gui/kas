// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS graphics backend over [WGPU]
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
//! See [`options::Options::load_from_env`] for documentation.
//!
//! [WGPU]: https://github.com/gfx-rs/wgpu

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod draw;
mod draw_shaded;
pub mod options;
mod shaded_theme;
mod surface;

use crate::draw::{CustomPipeBuilder, DrawPipe};
use kas::runner::{self, Result};
use wgpu::rwh;

pub use draw_shaded::{DrawShaded, DrawShadedImpl};
pub use options::Options;
pub use shaded_theme::ShadedTheme;
pub extern crate wgpu;

/// Graphics context
pub struct Instance<CB: CustomPipeBuilder> {
    options: Options,
    instance: wgpu::Instance,
    custom: CB,
}

impl<CB: CustomPipeBuilder> Instance<CB> {
    /// Construct a new `Instance`
    ///
    /// [`Options`] are typically default-constructed then
    /// [loaded from enviroment variables](Options::load_from_env).
    pub fn new(options: Options, custom: CB) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: options.backend(),
            ..Default::default()
        });

        Instance {
            options,
            instance,
            custom,
        }
    }
}

impl<CB: CustomPipeBuilder> runner::GraphicsInstance for Instance<CB> {
    type Shared = DrawPipe<CB::Pipe>;

    type Surface<'a> = surface::Surface<'a, CB::Pipe>;

    fn new_shared(&mut self, surface: Option<&Self::Surface<'_>>) -> Result<Self::Shared> {
        DrawPipe::new(
            &self.instance,
            &mut self.custom,
            &self.options,
            surface.map(|s| &s.surface),
        )
    }

    fn new_surface<'window, W>(
        &mut self,
        window: W,
        transparent: bool,
    ) -> Result<Self::Surface<'window>>
    where
        W: rwh::HasWindowHandle + rwh::HasDisplayHandle + Send + Sync + 'window,
        Self: Sized,
    {
        surface::Surface::new(&self.instance, window, transparent)
    }
}
