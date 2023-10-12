// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`Shell`] and supporting elements

use super::{GraphicalShell, Platform, ProxyAction, Result, SharedState};
use crate::config::Options;
use crate::draw::{DrawShared, DrawSharedImpl};
use crate::event;
use crate::theme::{self, Theme, ThemeConfig};
use crate::util::warn_about_error;
use crate::{impl_scope, AppData, Window, WindowId};
use std::cell::RefCell;
use std::rc::Rc;
use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopProxy};

/// The KAS shell
///
/// The "shell" is the layer over widgets, windows, events and graphics.
pub struct Shell<Data: AppData, G: GraphicalShell, T: Theme<G::Shared>> {
    el: EventLoop<ProxyAction>,
    windows: Vec<Box<super::Window<Data, G::Surface, T>>>,
    shared: SharedState<Data, G::Surface, T>,
}

impl_scope! {
    pub struct ShellBuilder<G: GraphicalShell, T: Theme<G::Shared>> {
        graphical: G,
        theme: T,
        options: Option<Options>,
        config: Option<Rc<RefCell<event::Config>>>,
    }

    impl Self {
        /// Construct from a graphical shell and a theme
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
        pub fn new(graphical: G, theme: T) -> Self {
            ShellBuilder {
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
        pub fn build<Data: AppData>(self, data: Data) -> Result<Shell<Data, G, T>> {
            let mut theme = self.theme;

            let options = self.options.unwrap_or_else(|| Options::from_env());
            options.init_theme_config(&mut theme)?;

            let config = self.config.unwrap_or_else(|| match options.read_config() {
                Ok(config) => Rc::new(RefCell::new(config)),
                Err(error) => {
                    warn_about_error("Shell::new_custom: failed to read config", &error);
                    Default::default()
                }
            });

            let el = EventLoopBuilder::with_user_event().build()?;

            let mut draw_shared = self.graphical.build()?;
            draw_shared.set_raster_config(theme.config().raster());

            let pw = PlatformWrapper(&el);
            let shared = SharedState::new(data, pw, draw_shared, theme, options, config)?;

            Ok(Shell {
                el,
                windows: vec![],
                shared,
            })
        }
    }
}

/// Shell associated types
///
/// Note: these could be inherent associated types of [`Shell`] when Rust#8995 is stable.
pub trait ShellAssoc {
    /// Shared draw state type
    type DrawShared: DrawSharedImpl;
}

impl<A: AppData, G: GraphicalShell, T> ShellAssoc for Shell<A, G, T>
where
    T: Theme<G::Shared> + 'static,
    T::Window: theme::Window,
{
    type DrawShared = G::Shared;
}

impl<Data: AppData, G> Shell<Data, G, G::DefaultTheme>
where
    G: GraphicalShell + Default,
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
    pub fn with_default_theme() -> ShellBuilder<G, G::DefaultTheme> {
        ShellBuilder::new(G::default(), G::DefaultTheme::default())
    }
}

impl<G, T> Shell<(), G, T>
where
    G: GraphicalShell + Default,
    T: Theme<G::Shared>,
{
    /// Construct a builder with the given `theme`
    #[inline]
    pub fn with_theme(theme: T) -> ShellBuilder<G, T> {
        ShellBuilder::new(G::default(), theme)
    }
}

impl<Data: AppData, G: GraphicalShell, T> Shell<Data, G, T>
where
    T: Theme<G::Shared> + 'static,
    T::Window: theme::Window,
{
    /// Access shared draw state
    #[inline]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        &mut self.shared.shell.draw
    }

    /// Access the theme by ref
    #[inline]
    pub fn theme(&self) -> &T {
        &self.shared.shell.theme
    }

    /// Access the theme by ref mut
    #[inline]
    pub fn theme_mut(&mut self) -> &mut T {
        &mut self.shared.shell.theme
    }

    /// Assume ownership of and display a window
    #[inline]
    pub fn add(&mut self, window: Window<Data>) -> WindowId {
        let id = self.shared.shell.next_window_id();
        let win = Box::new(super::Window::new(&self.shared.shell, id, window));
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
        let mut el = super::EventLoop::new(self.windows, self.shared);
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

    /// Guess scale factor of first window
    pub(super) fn guess_scale_factor(&self) -> f64 {
        if let Some(mon) = self.0.primary_monitor() {
            return mon.scale_factor();
        }
        if let Some(mon) = self.0.available_monitors().next() {
            return mon.scale_factor();
        }
        1.0
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

/// A proxy allowing control of a [`Shell`] from another thread.
///
/// Created by [`Shell::create_proxy`].
pub struct Proxy(EventLoopProxy<ProxyAction>);

/// Error type returned by [`Proxy`] functions.
///
/// This error occurs only if the [`Shell`] already terminated.
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
            .send_event(ProxyAction::Message(kas::erased::SendErased::new(msg)))
            .map_err(|_| ClosedError)
    }

    /// Wake async methods
    fn wake_async(&self) {
        // ignore error: if the loop closed the future has been dropped
        let _ = self.0.send_event(ProxyAction::WakeAsync);
    }
}
