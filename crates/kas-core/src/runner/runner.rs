// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`Runner`] and supporting elements

use super::{AppData, GraphicsBuilder, Platform, ProxyAction, Result, State};
use crate::config::{AutoFactory, Config, ConfigFactory};
use crate::draw::DrawSharedImpl;
use crate::theme::Theme;
use crate::{impl_scope, Window, WindowId, WindowIdFactory};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use winit::event_loop::{EventLoop, EventLoopProxy};

pub struct Runner<Data: AppData, G: GraphicsBuilder, T: Theme<G::Shared>> {
    config: Rc<RefCell<Config>>,
    config_writer: Option<Box<dyn FnMut(&Config)>>,
    data: Data,
    graphical: G,
    theme: T,
    el: EventLoop<ProxyAction>,
    platform: Platform,
    window_id_factory: WindowIdFactory,
    windows: Vec<Box<super::Window<Data, G, T>>>,
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
            let config = self.config.read_config()?;
            config.borrow_mut().init();

            self.theme.init(&config);

            let el = EventLoop::with_user_event().build()?;
            let platform = Platform::new(&el);

            Ok(Runner {
                config,
                config_writer: self.config.writer(),
                data,
                graphical: self.graphical,
                theme: self.theme,
                el,
                platform,
                window_id_factory: Default::default(),
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
    G: GraphicsBuilder + Default,
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
    G: GraphicsBuilder + Default,
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
        self.config.borrow()
    }

    /// Access config mutably
    #[inline]
    pub fn config_mut(&mut self) -> RefMut<Config> {
        self.config.borrow_mut()
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
        let id = self.window_id_factory.make_next();
        let win = Box::new(super::Window::new(
            self.config.clone(),
            self.platform,
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
        Proxy(self.el.create_proxy())
    }

    /// Run the main loop.
    #[inline]
    pub fn run(self) -> Result<()> {
        let state = State::new(
            self.platform,
            self.data,
            self.graphical,
            self.theme,
            self.config,
            self.config_writer,
            create_waker(&self.el),
            self.window_id_factory,
        )?;

        let mut l = super::Loop::new(self.windows, state);
        self.el.run_app(&mut l)?;
        Ok(())
    }
}

impl Platform {
    /// Get platform
    #[allow(clippy::needless_return)]
    fn new(el: &EventLoop<ProxyAction>) -> Platform {
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
                    use winit::platform::wayland::EventLoopExtWayland;
                    return if el.is_wayland() {
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
}

/// Create a waker
///
/// This waker may be used by a [`Future`](std::future::Future) to revive
/// event handling.
fn create_waker(el: &EventLoop<ProxyAction>) -> std::task::Waker {
    use std::sync::{Arc, Mutex};
    use std::task::{RawWaker, RawWakerVTable, Waker};

    // NOTE: Proxy is Send but not Sync. Mutex<T> is Sync for T: Send.
    // We wrap with Arc which is a Sync type supporting Clone and into_raw.
    type Data = Mutex<Proxy>;
    let proxy = Proxy(el.create_proxy());
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

/// A proxy allowing control of a UI from another thread.
///
/// Created by [`Runner::create_proxy`].
pub struct Proxy(EventLoopProxy<ProxyAction>);

/// Error type returned by [`Proxy`] functions.
///
/// This error occurs only if the [`Runner`] already terminated.
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
            .send_event(ProxyAction::Message(kas::messages::SendErased::new(msg)))
            .map_err(|_| ClosedError)
    }

    /// Wake async methods
    fn wake_async(&self) {
        // ignore error: if the loop closed the future has been dropped
        let _ = self.0.send_event(ProxyAction::WakeAsync);
    }
}
