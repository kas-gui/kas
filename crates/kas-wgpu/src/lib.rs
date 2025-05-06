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
use kas::runner;
use kas::theme::{FlatTheme, Theme};
use wgpu::rwh;

pub use draw_shaded::{DrawShaded, DrawShadedImpl};
pub use options::Options;
pub use shaded_theme::ShadedTheme;
pub extern crate wgpu;

/// Builder for a [`kas::runner::Runner`] using WGPU
pub struct Builder<CB: CustomPipeBuilder> {
    custom: CB,
    options: Options,
    read_env_vars: bool,
}

impl<CB: CustomPipeBuilder> runner::GraphicsBuilder for Builder<CB> {
    type DefaultTheme = FlatTheme;

    type Instance = wgpu::Instance;

    type Shared = DrawPipe<CB::Pipe>;

    type Surface<'a> = surface::Surface<'a, CB::Pipe>;

    fn new_instance(&mut self) -> Self::Instance {
        if self.read_env_vars {
            self.options.load_from_env();
        }

        wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: self.options.backend(),
            ..Default::default()
        })
    }

    fn build(self, instance: &Self::Instance) -> runner::Result<Self::Shared> {
        DrawPipe::new(instance, self.custom, &self.options)
    }

    fn new_surface<'window, W>(
        instance: &Self::Instance,
        window: W,
        transparent: bool,
    ) -> runner::Result<Self::Surface<'window>>
    where
        W: rwh::HasWindowHandle + rwh::HasDisplayHandle + Send + Sync + 'window,
        Self: Sized,
    {
        surface::Surface::new(instance, window, transparent)
    }
}

impl Default for Builder<()> {
    fn default() -> Self {
        Builder::new(())
    }
}

impl<CB: CustomPipeBuilder> Builder<CB> {
    /// Construct with the given pipe builder
    ///
    /// Pass `()` or use [`Self::default`] when not using a custom pipe.
    #[inline]
    pub fn new(cb: CB) -> Self {
        Builder {
            custom: cb,
            options: Options::default(),
            read_env_vars: true,
        }
    }

    /// Specify the default WGPU options
    ///
    /// These options serve as a default, but may still be replaced by values
    /// read from env vars unless disabled via [`Self::read_env_vars`].
    #[inline]
    pub fn with_wgpu_options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// En/dis-able reading options from environment variables
    ///
    /// Default: `true`. If enabled, options will be read from env vars where
    /// present (see [`Options::load_from_env`]).
    #[inline]
    pub fn read_env_vars(mut self, read_env_vars: bool) -> Self {
        self.read_env_vars = read_env_vars;
        self
    }

    /// Convert to a [`runner::Builder`] using the default theme
    #[inline]
    pub fn with_default_theme(self) -> runner::Builder<Self, FlatTheme, kas::config::AutoFactory> {
        runner::Builder::new(self, FlatTheme::new())
    }

    /// Convert to a [`runner::Builder`] using the specified `theme`
    #[inline]
    pub fn with_theme<T>(self, theme: T) -> runner::Builder<Self, T, kas::config::AutoFactory>
    where
        T: Theme<DrawPipe<CB::Pipe>>,
    {
        runner::Builder::new(self, theme)
    }
}
