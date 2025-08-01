// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`Runner`], platforms and backends
//!
//! Start by constructing a [`Runner`] or its [`Default`](type@Default)
//! type-def (requires a backend be enabled, e.g. "wgpu").

use crate::Window;
use crate::WindowId;
use crate::config::{AutoFactory, Config, ConfigFactory};
use crate::draw::DrawSharedImpl;
use crate::theme::Theme;
pub use kas_core::runner::{AppData, ClosedError, Error, MessageStack, Platform, Proxy, Result};
use kas_core::runner::{GraphicsInstance, PreLaunchState};
use kas_core::theme::FlatTheme;
use kas_wgpu::draw::CustomPipeBuilder;
use std::cell::{Ref, RefMut};

/// Builder for a [`Runner`]'s graphics instance
#[cfg(feature = "wgpu")]
pub struct WgpuBuilder<CB: CustomPipeBuilder> {
    custom: CB,
    options: kas_wgpu::Options,
    read_env_vars: bool,
}

#[cfg(feature = "wgpu")]
impl<CB: CustomPipeBuilder> WgpuBuilder<CB> {
    /// Construct with the given pipe builder
    ///
    /// Pass `()` or use [`Self::default`] when not using a custom pipe.
    #[inline]
    fn new(cb: CB) -> Self {
        WgpuBuilder {
            custom: cb,
            options: kas_wgpu::Options::default(),
            read_env_vars: true,
        }
    }

    /// Specify the default WGPU options
    ///
    /// These options serve as a default, but may still be replaced by values
    /// read from env vars unless disabled via [`Self::read_env_vars`].
    #[inline]
    pub fn with_wgpu_options(mut self, options: kas_wgpu::Options) -> Self {
        self.options = options;
        self
    }

    /// En/dis-able reading options from environment variables
    ///
    /// Default: `true`. If enabled, options will be read from env vars where
    /// present (see [`kas_wgpu::Options::load_from_env`]).
    #[inline]
    pub fn read_env_vars(mut self, read_env_vars: bool) -> Self {
        self.read_env_vars = read_env_vars;
        self
    }

    /// Use a selected theme
    #[inline]
    pub fn with_default_theme(self) -> Builder<FlatTheme, kas_wgpu::Instance<CB>> {
        self.with_theme(FlatTheme::new())
    }

    /// Use a specified theme
    #[inline]
    pub fn with_theme<T>(mut self, theme: T) -> Builder<T, kas_wgpu::Instance<CB>>
    where
        T: Theme<<kas_wgpu::Instance<CB> as GraphicsInstance>::Shared>,
    {
        if self.read_env_vars {
            self.options.load_from_env();
        }

        Builder {
            graphics: kas_wgpu::Instance::new(self.options, self.custom),
            theme,
            config: AutoFactory::default(),
        }
    }
}

/// Builder for a [`Runner`]
#[derive(Default)]
pub struct Builder<T = FlatTheme, G = kas_wgpu::Instance<()>, C = AutoFactory>
where
    T: Theme<G::Shared> + 'static,
    G: GraphicsInstance,
    C: ConfigFactory,
{
    graphics: G,
    theme: T,
    config: C,
}

impl<T: Theme<G::Shared>, G: GraphicsInstance, C: ConfigFactory> Builder<T, G, C> {
    /// Use the specified [`ConfigFactory`]
    #[inline]
    pub fn with_config<CF: ConfigFactory>(self, config: CF) -> Builder<T, G, CF> {
        Builder {
            graphics: self.graphics,
            theme: self.theme,
            config,
        }
    }

    /// Build with `data`
    pub fn build<Data: AppData>(mut self, data: Data) -> Result<Runner<Data, T, G>> {
        let state = PreLaunchState::new(self.config)?;

        self.theme.init(state.config());

        Ok(Runner {
            data,
            graphics: self.graphics,
            theme: self.theme,
            state,
            windows: vec![],
        })
    }
}

/// Runner pre-launch state
///
/// Suggested construction patterns:
///
/// -   <code>kas::runner::[Runner](type@Runner)::[new](Runner::new)(data)?</code>
/// -   <code>kas::runner::[Runner](type@Runner)::[with_theme](Runner::with_theme)(theme).[build](Builder::build)(data)?</code>
/// -   <code>kas::runner::[Runner](type@Runner)::[with_wgpu_pipe](Runner::with_wgpu_pipe)(custom_wgpu_pipe).[with_theme](WgpuBuilder::with_theme)(theme).[build](Builder::build)(data)?</code>
///
/// Where:
///
/// -   `data` is `()` or some object implementing [`AppData`]
/// -   `theme` is some object implementing [`Theme`]
/// -   `custom_wgpu_pipe` is a custom WGPU graphics pipeline
pub struct Runner<
    Data: AppData,
    T: Theme<G::Shared> = FlatTheme,
    G: GraphicsInstance = kas_wgpu::Instance<()>,
> {
    data: Data,
    graphics: G,
    state: PreLaunchState,
    theme: T,
    windows: Vec<Box<kas_core::runner::Window<Data, G, T>>>,
}

/// Inherenet associated types of [`Runner`]
///
/// Note: these could be inherent associated types of [`Runner`] when Rust#8995 is stable.
pub trait RunnerInherent {
    /// Shared draw state type
    type DrawShared: DrawSharedImpl;
}

impl<A: AppData, G: GraphicsInstance, T> RunnerInherent for Runner<A, T, G>
where
    T: Theme<G::Shared> + 'static,
{
    type DrawShared = G::Shared;
}

impl<Data: AppData> Runner<Data> {
    /// Construct a new instance with default options and theme
    ///
    /// All user interfaces are expected to provide `data: Data`: widget data
    /// shared across all windows. If not required this may be `()`.
    ///
    /// Configuration is supplied by [`AutoFactory`].
    #[inline]
    pub fn new(data: Data) -> Result<Self> {
        WgpuBuilder::new(())
            .with_theme(Default::default())
            .build(data)
    }
}

impl<T: Theme<kas_wgpu::draw::DrawPipe<()>>> Runner<(), T> {
    /// Construct a builder with the given `theme`
    #[inline]
    pub fn with_theme(theme: T) -> Builder<T> {
        WgpuBuilder::new(()).with_theme(theme)
    }
}

impl Runner<()> {
    /// Construct a builder with the default theme
    #[inline]
    pub fn with_default_theme() -> Builder {
        WgpuBuilder::new(()).with_theme(Default::default())
    }

    /// Build with a custom WGPU pipe
    #[cfg(feature = "wgpu")]
    pub fn with_wgpu_pipe<CB: CustomPipeBuilder>(cb: CB) -> WgpuBuilder<CB> {
        WgpuBuilder::new(cb)
    }
}

impl<Data: AppData, G: GraphicsInstance, T> Runner<Data, T, G>
where
    T: Theme<G::Shared> + 'static,
{
    /// Access config
    #[inline]
    pub fn config(&self) -> Ref<'_, Config> {
        self.state.config().borrow()
    }

    /// Access config mutably
    #[inline]
    pub fn config_mut(&mut self) -> RefMut<'_, Config> {
        self.state.config().borrow_mut()
    }

    /// Access the theme by ref
    #[inline]
    pub fn theme(&self) -> &T {
        &self.theme
    }

    /// Access the theme by ref mut
    #[inline]
    pub fn theme_mut(&mut self) -> &mut T {
        &mut self.theme
    }

    /// Assume ownership of and display a window
    #[inline]
    pub fn add(&mut self, window: Window<Data>) -> WindowId {
        let id = self.state.next_window_id();
        let win = Box::new(kas_core::runner::Window::new(
            self.state.config().clone(),
            self.state.platform(),
            id,
            window,
        ));
        self.windows.push(win);
        id
    }

    /// Assume ownership of and display a window, inline
    #[inline]
    pub fn with(mut self, window: Window<Data>) -> Self {
        let _ = self.add(window);
        self
    }

    /// Create a proxy which can be used to update the UI from another thread
    pub fn create_proxy(&self) -> Proxy {
        self.state.create_proxy()
    }

    /// Run the main loop.
    #[inline]
    pub fn run(self) -> Result<()> {
        self.state
            .run(self.data, self.graphics, self.theme, self.windows)
    }
}
