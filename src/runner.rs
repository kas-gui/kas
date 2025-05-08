// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`Runner`], platforms and backends
//!
//! Start by constructing a [`Runner`] or its [`Default`](type@Default)
//! type-def (requires a backend be enabled, e.g. "wgpu").

pub use kas_core::runner::*;

use crate::config::{AutoFactory, Config, ConfigFactory};
use crate::draw::DrawSharedImpl;
use crate::theme::Theme;
use crate::{impl_scope, WindowId};
use std::cell::{Ref, RefMut};

/// Runner pre-launch state
///
/// Suggested construction patterns:
///
/// -   <code>kas::runner::[Default](type@Default)::[new](Runner::new)(data)?</code>
/// -   <code>kas::runner::[Default](type@Default)::[with_theme](Runner::with_theme)(theme).[build](Builder::build)(data)?</code>
///
/// Where:
///
/// -   `data` is `()` or some object implementing [`AppData`]
/// -   `theme` is some object implementing [`Theme`]
/// -   `custom_wgpu_pipe` is a custom WGPU graphics pipeline
pub struct Runner<Data: AppData, G: GraphicsBuilder, T: Theme<G::Shared>> {
    data: Data,
    graphical: G,
    state: PreLaunchState,
    theme: T,
    windows: Vec<Box<Window<Data, G, T>>>,
}

impl_scope! {
    pub struct Builder<G: GraphicsBuilder, T: Theme<G::Shared>, C: ConfigFactory> {
        graphical: G,
        theme: T,
        config: C,
    }

    impl<G: GraphicsBuilder, T: Theme<G::Shared>> Builder<G, T, AutoFactory> {
        /// Construct from a graphics backend and a theme
        ///
        /// Configuration uses [`AutoFactory`]. Call [`Self::with_config`] to override.
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
        pub fn new(graphical: G, theme: T) -> Self {
            Builder {
                graphical,
                theme,
                config: AutoFactory::default(),
            }
        }
    }

    impl Self {
        /// Use the specified [`ConfigFactory`]
        #[inline]
        pub fn with_config<CF: ConfigFactory>(self, config: CF) -> Builder<G, T, CF> {
            Builder {
                graphical: self.graphical,
                theme: self.theme,
                config,
            }
        }

        /// Build with `data`
        pub fn build<Data: AppData>(mut self, data: Data) -> Result<Runner<Data, G, T>> {
            let state = PreLaunchState::new(self.config)?;

            self.theme.init(state.config());

            Ok(Runner {
                data,
                graphical: self.graphical,
                theme: self.theme,
                state,
                windows: vec![],
            })
        }
    }
}

/// Inherenet associated types of [`Runner`]
///
/// Note: these could be inherent associated types of [`Runner`] when Rust#8995 is stable.
pub trait RunnerInherent {
    /// Shared draw state type
    type DrawShared: DrawSharedImpl;
}

impl<A: AppData, G: GraphicsBuilder, T> RunnerInherent for Runner<A, G, T>
where
    T: Theme<G::Shared> + 'static,
{
    type DrawShared = G::Shared;
}

impl<Data: AppData, G> Runner<Data, G, G::DefaultTheme>
where
    G: GraphicsBuilder + std::default::Default,
{
    /// Construct a new instance with default options and theme
    ///
    /// All user interfaces are expected to provide `data: Data`: widget data
    /// shared across all windows. If not required this may be `()`.
    ///
    /// Configuration is supplied by [`AutoFactory`].
    #[inline]
    pub fn new(data: Data) -> Result<Self> {
        Self::with_default_theme().build(data)
    }

    /// Construct a builder with the default theme
    #[inline]
    pub fn with_default_theme() -> Builder<G, G::DefaultTheme, AutoFactory> {
        Builder::new(G::default(), G::DefaultTheme::default())
    }
}

impl<G, T> Runner<(), G, T>
where
    G: GraphicsBuilder + std::default::Default,
    T: Theme<G::Shared>,
{
    /// Construct a builder with the given `theme`
    #[inline]
    pub fn with_theme(theme: T) -> Builder<G, T, AutoFactory> {
        Builder::new(G::default(), theme)
    }
}

impl<Data: AppData, G: GraphicsBuilder, T> Runner<Data, G, T>
where
    T: Theme<G::Shared> + 'static,
{
    /// Access config
    #[inline]
    pub fn config(&self) -> Ref<Config> {
        self.state.config().borrow()
    }

    /// Access config mutably
    #[inline]
    pub fn config_mut(&mut self) -> RefMut<Config> {
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
    pub fn add(&mut self, window: crate::Window<Data>) -> WindowId {
        let id = self.state.next_window_id();
        let win = Box::new(Window::new(
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
    pub fn with(mut self, window: crate::Window<Data>) -> Self {
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
            .run(self.data, self.graphical, self.theme, self.windows)
    }
}

/// Runner pre-launch state, configured with the default graphics backend
#[cfg(feature = "wgpu")]
pub type Default<Data, T = crate::theme::FlatTheme> = Runner<Data, kas_wgpu::Builder<()>, T>;
