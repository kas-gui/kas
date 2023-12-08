// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`Application`] and supporting elements

use super::{AppData, AppGraphicsBuilder, AppState, Platform, ProxyAction, Result};
use crate::config::Options;
use crate::draw::{DrawShared, DrawSharedImpl};
use crate::event;
use crate::theme::{self, Theme, ThemeConfig};
use crate::util::warn_about_error;
use crate::{impl_scope, Window, WindowId};
use std::cell::RefCell;
use std::rc::Rc;
use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopProxy};

pub struct Application<Data: AppData, G: AppGraphicsBuilder, T: Theme<G::Shared>> {
    el: EventLoop<ProxyAction>,
    windows: Vec<Box<super::Window<Data, G::Surface, T>>>,
    state: AppState<Data, G::Surface, T>,
}

impl_scope! {
    pub struct AppBuilder<G: AppGraphicsBuilder, T: Theme<G::Shared>> {
        graphical: G,
        theme: T,
        options: Option<Options>,
        config: Option<Rc<RefCell<event::Config>>>,
    }

    impl Self {
        /// Construct from a graphics backend and a theme
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
        pub fn new(graphical: G, theme: T) -> Self {
            AppBuilder {
                graphical,
                theme,
                options: None,
                config: None,
            }
        }

        /// Use the specified `options`
        ///
        /// If omitted, options are provided by [`Options::from_env`].
        #[inline]
        pub fn with_options(mut self, options: Options) -> Self {
            self.options = Some(options);
            self
        }

        /// Use the specified event `config`
        ///
        /// This is a wrapper around [`Self::with_event_config_rc`].
        ///
        /// If omitted, config is provided by [`Options::read_config`].
        #[inline]
        pub fn with_event_config(self, config: event::Config) -> Self {
            self.with_event_config_rc(Rc::new(RefCell::new(config)))
        }

        /// Use the specified event `config`
        ///
        /// If omitted, config is provided by [`Options::read_config`].
        #[inline]
        pub fn with_event_config_rc(mut self, config: Rc<RefCell<event::Config>>) -> Self {
            self.config = Some(config);
            self
        }

        /// Build with `data`
        pub fn build<Data: AppData>(self, data: Data) -> Result<Application<Data, G, T>> {
            let mut theme = self.theme;

            let options = self.options.unwrap_or_else(Options::from_env);
            options.init_theme_config(&mut theme)?;

            let config = self.config.unwrap_or_else(|| match options.read_config() {
                Ok(config) => Rc::new(RefCell::new(config)),
                Err(error) => {
                    warn_about_error("AppBuilder::build: failed to read config", &error);
                    Default::default()
                }
            });

            let el = EventLoopBuilder::with_user_event().build()?;

            let mut draw_shared = self.graphical.build()?;
            draw_shared.set_raster_config(theme.config().raster());

            let pw = PlatformWrapper(&el);
            let state = AppState::new(data, pw, draw_shared, theme, options, config)?;

            Ok(Application {
                el,
                windows: vec![],
                state,
            })
        }
    }
}

/// Application associated types
///
/// Note: these could be inherent associated types of [`Application`] when Rust#8995 is stable.
pub trait AppAssoc {
    /// Shared draw state type
    type DrawShared: DrawSharedImpl;
}

impl<A: AppData, G: AppGraphicsBuilder, T> AppAssoc for Application<A, G, T>
where
    T: Theme<G::Shared> + 'static,
    T::Window: theme::Window,
{
    type DrawShared = G::Shared;
}

impl<Data: AppData, G> Application<Data, G, G::DefaultTheme>
where
    G: AppGraphicsBuilder + Default,
{
    /// Construct a new instance with default options and theme
    ///
    /// All user interfaces are expected to provide `data: Data`: widget data
    /// shared across all windows. If not required this may be `()`.
    ///
    /// Environment variables may affect option selection; see documentation
    /// of [`Options::from_env`]. KAS config is provided by
    /// [`Options::read_config`].
    #[inline]
    pub fn new(data: Data) -> Result<Self> {
        Self::with_default_theme().build(data)
    }

    /// Construct a builder with the default theme
    #[inline]
    pub fn with_default_theme() -> AppBuilder<G, G::DefaultTheme> {
        AppBuilder::new(G::default(), G::DefaultTheme::default())
    }
}

impl<G, T> Application<(), G, T>
where
    G: AppGraphicsBuilder + Default,
    T: Theme<G::Shared>,
{
    /// Construct a builder with the given `theme`
    #[inline]
    pub fn with_theme(theme: T) -> AppBuilder<G, T> {
        AppBuilder::new(G::default(), theme)
    }
}

impl<Data: AppData, G: AppGraphicsBuilder, T> Application<Data, G, T>
where
    T: Theme<G::Shared> + 'static,
    T::Window: theme::Window,
{
    /// Access shared draw state
    #[inline]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        &mut self.state.shared.draw
    }

    /// Access the theme by ref
    #[inline]
    pub fn theme(&self) -> &T {
        &self.state.shared.theme
    }

    /// Access the theme by ref mut
    #[inline]
    pub fn theme_mut(&mut self) -> &mut T {
        &mut self.state.shared.theme
    }

    /// Assume ownership of and display a window
    #[inline]
    pub fn add(&mut self, window: Window<Data>) -> WindowId {
        let id = self.state.shared.next_window_id();
        let win = Box::new(super::Window::new(&self.state.shared, id, window));
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
        Proxy(self.el.create_proxy())
    }

    /// Run the main loop.
    #[inline]
    pub fn run(self) -> Result<()> {
        let mut el = super::EventLoop::new(self.windows, self.state);
        self.el.run(move |event, elwt| el.handle(event, elwt))?;
        Ok(())
    }
}

pub(super) struct PlatformWrapper<'a>(&'a EventLoop<ProxyAction>);
impl<'a> PlatformWrapper<'a> {
    /// Get platform
    #[allow(clippy::needless_return)]
    pub(super) fn platform(&self) -> Platform {
        // Logic copied from winit::platform_impl module.

        #[cfg(target_os = "windows")]
        return Platform::Windows;

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            cfg_if::cfg_if! {
                if #[cfg(all(feature = "wayland", feature = "x11"))] {
                    use winit::platform::wayland::EventLoopWindowTargetExtWayland;
                    return if self.0.is_wayland() {
                        Platform::Wayland
                    } else {
                        Platform::X11
                    };
                } else if #[cfg(feature = "wayland")] {
                    return Platform::Wayland;
                } else if #[cfg(feature = "x11")] {
                    return Platform::X11;
                } else {
                    compile_error!("Please select a feature to build for unix: `x11`, `wayland`");
                }
            }
        }

        #[cfg(target_os = "macos")]
        return Platform::MacOS;

        #[cfg(target_os = "android")]
        return Platform::Android;

        #[cfg(target_os = "ios")]
        return Platform::IOS;

        #[cfg(target_arch = "wasm32")]
        return Platform::Web;

        // Otherwise platform is unsupported!
    }

    /// Create a waker
    ///
    /// This waker may be used by a [`Future`](std::future::Future) to revive
    /// event handling.
    pub(super) fn create_waker(&self) -> std::task::Waker {
        use std::sync::{Arc, Mutex};
        use std::task::{RawWaker, RawWakerVTable, Waker};

        // NOTE: Proxy is Send but not Sync. Mutex<T> is Sync for T: Send.
        // We wrap with Arc which is a Sync type supporting Clone and into_raw.
        type Data = Mutex<Proxy>;
        let proxy = Proxy(self.0.create_proxy());
        let a: Arc<Data> = Arc::new(Mutex::new(proxy));
        let data = Arc::into_raw(a);

        const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

        unsafe fn clone(data: *const ()) -> RawWaker {
            let a = Arc::from_raw(data as *const Data);
            let c = Arc::into_raw(a.clone());
            let _do_not_drop = Arc::into_raw(a);
            RawWaker::new(c as *const (), &VTABLE)
        }
        unsafe fn wake(data: *const ()) {
            let a = Arc::from_raw(data as *const Data);
            a.lock().unwrap().wake_async();
        }
        unsafe fn wake_by_ref(data: *const ()) {
            let a = Arc::from_raw(data as *const Data);
            a.lock().unwrap().wake_async();
            let _do_not_drop = Arc::into_raw(a);
        }
        unsafe fn drop(data: *const ()) {
            let _ = Arc::from_raw(data as *const Data);
        }

        let raw_waker = RawWaker::new(data as *const (), &VTABLE);
        unsafe { Waker::from_raw(raw_waker) }
    }
}

/// A proxy allowing control of an application from another thread.
///
/// Created by [`Application::create_proxy`].
pub struct Proxy(EventLoopProxy<ProxyAction>);

/// Error type returned by [`Proxy`] functions.
///
/// This error occurs only if the application already terminated.
pub struct ClosedError;

impl Proxy {
    /// Close a specific window.
    pub fn close(&self, id: WindowId) -> std::result::Result<(), ClosedError> {
        self.0
            .send_event(ProxyAction::Close(id))
            .map_err(|_| ClosedError)
    }

    /// Close all windows and terminate the UI.
    pub fn close_all(&self) -> std::result::Result<(), ClosedError> {
        self.0
            .send_event(ProxyAction::CloseAll)
            .map_err(|_| ClosedError)
    }

    /// Send a message to [`AppData`]
    ///
    /// This is similar to [`EventCx::push`](crate::event::EventCx::push),
    /// but can only be handled by top-level [`AppData`].
    pub fn push<M: std::fmt::Debug + Send + 'static>(
        &mut self,
        msg: M,
    ) -> std::result::Result<(), ClosedError> {
        self.0
            .send_event(ProxyAction::Message(kas::message::SendErased::new(msg)))
            .map_err(|_| ClosedError)
    }

    /// Wake async methods
    fn wake_async(&self) {
        // ignore error: if the loop closed the future has been dropped
        let _ = self.0.send_event(ProxyAction::WakeAsync);
    }
}
