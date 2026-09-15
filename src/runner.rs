// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`Runner`], platforms and backends
//!
//! Start by constructing a [`Runner`] or its [`Default`](type@Default)
//! type-def (requires a backend be enabled, e.g. "wgpu").

use crate::config::{AutoFactory, Config, ConfigFactory};
use crate::theme::Theme;
use crate::window::{Window, WindowId};
pub use kas_core::runner::{AppData, ClosedError, Error, Platform, Proxy, ReadMessage, Result};
use kas_core::runner::{GraphicsInstance, PreLaunchState};
#[allow(unused)]
use kas_core::theme::{FlatTheme, SimpleTheme};
use kas_core::winit::event_loop::EventLoop;
#[cfg(feature = "wgpu")]
use kas_wgpu::draw::CustomPipeBuilder;
use std::cell::{Ref, RefMut};

#[cfg(not(any(feature = "wgpu", feature = "soft")))]
compile_error!("At least one of the following features must be enabled: wgpu, soft");

pub trait GraphicsBackend {
    type Instance: GraphicsInstance + 'static;

    type DefaultTheme: Theme<<Self::Instance as GraphicsInstance>::Shared> + Default;

    #[doc(hidden)]
    fn into_instance(self) -> Self::Instance;
}

#[cfg(feature = "wgpu")]
#[derive(Debug, Default)]
pub struct WgpuBackend<CB: CustomPipeBuilder> {
    custom: CB,
    options: kas_wgpu::Options,
    read_env_vars: bool,
}
#[cfg(feature = "wgpu")]
impl<CB: CustomPipeBuilder> GraphicsBackend for WgpuBackend<CB> {
    type Instance = kas_wgpu::Instance<CB>;

    type DefaultTheme = FlatTheme;

    fn into_instance(mut self) -> Self::Instance {
        if self.read_env_vars {
            self.options.load_from_env();
        }

        kas_wgpu::Instance::new(self.options, self.custom)
    }
}

#[cfg(feature = "soft")]
#[derive(Debug, Default)]
pub struct SoftBackend;
#[cfg(feature = "soft")]
impl GraphicsBackend for SoftBackend {
    type Instance = kas_soft::Instance;

    type DefaultTheme = SimpleTheme;

    fn into_instance(self) -> Self::Instance {
        kas_soft::Instance::new()
    }
}

#[cfg(feature = "wgpu")]
pub type DefaultBackend = WgpuBackend<()>;
#[cfg(all(not(feature = "wgpu"), feature = "soft"))]
pub type DefaultBackend = SoftBackend;

/// First-stage builder for a [`Runner`]
#[derive(Debug)]
pub struct BackendBuilder<B: GraphicsBackend>(B, EventLoop);

impl<B: GraphicsBackend + Default> BackendBuilder<B> {
    /// Try constructing an instance
    #[inline]
    pub fn new() -> Result<Self> {
        let el = EventLoop::new()?;
        Ok(BackendBuilder(B::default(), el))
    }
}

#[cfg(feature = "wgpu")]
impl<CB: CustomPipeBuilder> BackendBuilder<WgpuBackend<CB>> {
    /// Use a `custom` pipe builder
    #[inline]
    pub fn with_custom_pipe<CB2: CustomPipeBuilder>(
        self,
        custom: CB2,
    ) -> BackendBuilder<WgpuBackend<CB2>> {
        BackendBuilder(
            WgpuBackend {
                custom,
                options: self.0.options,
                read_env_vars: self.0.read_env_vars,
            },
            self.1,
        )
    }

    /// Specify the default WGPU options
    ///
    /// These options serve as a default, but may still be replaced by values
    /// read from env vars unless disabled via [`Self::read_env_vars`].
    #[inline]
    pub fn with_wgpu_options(mut self, options: kas_wgpu::Options) -> Self {
        self.0.options = options;
        self
    }

    /// En/dis-able reading options from environment variables
    ///
    /// Default: `true`. If enabled, options will be read from env vars where
    /// present (see [`kas_wgpu::Options::load_from_env`]).
    #[inline]
    pub fn read_env_vars(mut self, read_env_vars: bool) -> Self {
        self.0.read_env_vars = read_env_vars;
        self
    }
}

impl<B: GraphicsBackend> BackendBuilder<B> {
    /// Use a selected theme
    #[inline]
    pub fn with_default_theme(self) -> Builder<B, B::DefaultTheme> {
        self.with_theme(B::DefaultTheme::default())
    }

    /// Use a specified theme
    #[inline]
    pub fn with_theme<T>(self, theme: T) -> Builder<B, T>
    where
        T: Theme<<B::Instance as GraphicsInstance>::Shared>,
    {
        Builder {
            graphics: self.0.into_instance(),
            el: self.1,
            theme,
            config: AutoFactory::default(),
        }
    }
}

/// Second-stage builder for a [`Runner`]
pub struct Builder<B, T, C = AutoFactory>
where
    B: GraphicsBackend,
    T: Theme<<B::Instance as GraphicsInstance>::Shared> + 'static,
    C: ConfigFactory,
{
    graphics: B::Instance,
    el: EventLoop,
    theme: T,
    config: C,
}

impl<B: GraphicsBackend, T: Theme<<B::Instance as GraphicsInstance>::Shared>, C: ConfigFactory>
    Builder<B, T, C>
{
    /// Use the specified [`ConfigFactory`]
    #[inline]
    pub fn with_config<CF: ConfigFactory>(self, config: CF) -> Builder<B, T, CF> {
        Builder {
            graphics: self.graphics,
            el: self.el,
            theme: self.theme,
            config,
        }
    }

    /// Build with `data`
    pub fn build<Data: AppData>(mut self, data: Data) -> Result<Runner<Data, B, T>> {
        let state = PreLaunchState::new(self.config, self.el)?;

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
/// -   <code>kas::runner::[Runner](type@Runner)::[with_theme](Runner::with_theme)(theme)?.[build](Builder::build)(data)?</code>
/// -   <code>kas::runner::[Runner](type@Runner)::[builder](Runner::builder)()?.[with_default_theme](BackendBuilder::with_default_theme)().[build](Builder::build)(data)?</code>
/// -   <code>kas::runner::[Runner](type@Runner)::[builder](Runner::builder)()?.[with_theme](BackendBuilder::with_theme)(theme).[build](Builder::build)(data)?</code>
///
/// Where:
///
/// -   `data` is `()` or some object implementing [`AppData`]
/// -   `theme` is some object implementing [`Theme`]
/// -   `custom_wgpu_pipe` is a custom WGPU graphics pipeline
pub struct Runner<
    Data: AppData,
    B: GraphicsBackend = DefaultBackend,
    T: Theme<<B::Instance as GraphicsInstance>::Shared> = <B as GraphicsBackend>::DefaultTheme,
> {
    data: Data,
    graphics: B::Instance,
    state: PreLaunchState,
    theme: T,
    windows: Vec<Box<kas_core::runner::Window<Data, B::Instance, T>>>,
}

impl<Data: AppData> Runner<Data> {
    /// Construct a new instance with default options and theme
    ///
    /// All user interfaces are expected to provide `data: Data`: widget data
    /// shared across all windows. If not required this may be `()`.
    ///
    /// Configuration is supplied by [`AutoFactory`].
    ///
    /// To use non-default options instead use [`Self::builder`] or [`Self::with_theme`].
    #[inline]
    pub fn new(data: Data) -> Result<Self> {
        BackendBuilder::new()?.with_default_theme().build(data)
    }
}

impl Runner<()> {
    /// Construct a first-stage builder
    #[inline]
    pub fn builder() -> Result<BackendBuilder<DefaultBackend>> {
        BackendBuilder::<DefaultBackend>::new()
    }

    /// Construct a second-stage builder with the given `theme`
    #[inline]
    pub fn with_theme<T>(theme: T) -> Result<Builder<DefaultBackend, T>>
    where
        T: Theme<<<DefaultBackend as GraphicsBackend>::Instance as GraphicsInstance>::Shared>,
    {
        BackendBuilder::new().map(|b| b.with_theme(theme))
    }
}

impl<Data: AppData, B: GraphicsBackend, T> Runner<Data, B, T>
where
    T: Theme<<B::Instance as GraphicsInstance>::Shared> + 'static,
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
            window.boxed(),
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
