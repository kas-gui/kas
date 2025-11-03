// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`Runner`] and supporting elements

use super::{AppData, GraphicsInstance, Platform, ProxyAction, Result, Shared};
use crate::config::{Config, ConfigFactory};
#[allow(unused)] use crate::event::ConfigCx;
use crate::theme::Theme;
use crate::window::{WindowId, WindowIdFactory};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use winit::event_loop::{EventLoop, EventLoopProxy};

/// State used to launch the UI
///
/// This is a low-level type; it is recommended to instead use
/// [`Runner`](https://docs.rs/kas/latest/kas/runner/struct.Runner.html).
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub struct PreLaunchState {
    config: Rc<RefCell<Config>>,
    config_writer: Option<Box<dyn FnMut(&Config)>>,
    el: EventLoop<()>,
    platform: Platform,
    proxy_tx: mpsc::SyncSender<ProxyAction>,
    proxy_rx: mpsc::Receiver<ProxyAction>,
    window_id_factory: WindowIdFactory,
}

impl PreLaunchState {
    /// Construct
    pub fn new<C: ConfigFactory>(config: C) -> Result<Self> {
        let mut cf = config;
        let config = cf.read_config()?;
        config.borrow_mut().init();

        let el = EventLoop::with_user_event().build()?;
        let platform = Platform::new(&el);

        let (proxy_tx, proxy_rx) = mpsc::sync_channel(16);

        Ok(PreLaunchState {
            config,
            config_writer: cf.writer(),
            el,
            platform,
            proxy_tx,
            proxy_rx,
            window_id_factory: Default::default(),
        })
    }

    /// Access config
    #[inline]
    pub fn config(&self) -> &Rc<RefCell<Config>> {
        &self.config
    }

    /// Generate a [`WindowId`]
    #[inline]
    pub fn next_window_id(&mut self) -> WindowId {
        self.window_id_factory.make_next()
    }

    /// Get the platform
    #[inline]
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Create a proxy which can be used to update the UI from another thread
    pub fn create_proxy(&self) -> Proxy {
        Proxy::new(self.proxy_tx.clone(), self.el.create_proxy())
    }

    /// Run the main loop
    pub fn run<Data: AppData, G: GraphicsInstance, T: Theme<G::Shared>>(
        self,
        data: Data,
        graphical: G,
        theme: T,
        windows: Vec<Box<super::Window<Data, G, T>>>,
    ) -> Result<()> {
        let shared = Shared::<Data, _, _>::new(
            self.platform,
            graphical,
            theme,
            self.config,
            self.config_writer,
            create_waker(&self.el),
            #[cfg(feature = "accesskit")]
            self.el.create_proxy(),
            self.proxy_rx,
            self.window_id_factory,
        )?;

        let mut l = super::Loop::new(windows, shared, data);
        self.el.run_app(&mut l)?;
        Ok(())
    }
}

impl Platform {
    /// Get platform
    #[allow(clippy::needless_return)]
    fn new(_el: &EventLoop<()>) -> Platform {
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
                    return if _el.is_wayland() {
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
fn create_waker(el: &EventLoop<()>) -> std::task::Waker {
    use std::sync::Arc;
    use std::task::{RawWaker, RawWakerVTable, Waker};

    // We wrap with Arc which is a Sync type supporting Clone and into_raw.
    type Data = EventLoopProxy<()>;
    let proxy = el.create_proxy();
    let a: Arc<Data> = Arc::new(proxy);
    let data = Arc::into_raw(a);

    fn wake_async(proxy: &Data) {
        // ignore error: if the loop closed the future has been dropped
        let _ = proxy.send_event(());
    }

    const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    unsafe fn clone(data: *const ()) -> RawWaker {
        unsafe {
            let a = Arc::from_raw(data as *const Data);
            let c = Arc::into_raw(a.clone());
            let _do_not_drop = Arc::into_raw(a);
            RawWaker::new(c as *const (), &VTABLE)
        }
    }
    unsafe fn wake(data: *const ()) {
        unsafe {
            let a = Arc::from_raw(data as *const Data);
            wake_async(&a);
        }
    }
    unsafe fn wake_by_ref(data: *const ()) {
        unsafe {
            let a = Arc::from_raw(data as *const Data);
            wake_async(&a);
            let _do_not_drop = Arc::into_raw(a);
        }
    }
    unsafe fn drop(data: *const ()) {
        unsafe {
            let _ = Arc::from_raw(data as *const Data);
        }
    }

    let raw_waker = RawWaker::new(data as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw_waker) }
}

/// A proxy allowing control of a UI from another thread.
///
/// Created by [`Runner::create_proxy`](https://docs.rs/kas/latest/kas/runner/struct.Runner.html#method.create_proxy).
#[derive(Clone)]
pub struct Proxy {
    tx: mpsc::SyncSender<ProxyAction>,
    waker: EventLoopProxy<()>,
}

/// Error type returned by [`Proxy`] functions.
///
/// This error occurs only if the [`Runner`](https://docs.rs/kas/latest/kas/runner/struct.Runner.html) already terminated.
pub struct ClosedError;

impl Proxy {
    #[inline]
    fn new(tx: mpsc::SyncSender<ProxyAction>, waker: EventLoopProxy<()>) -> Self {
        Proxy { tx, waker }
    }

    /// Close a specific window.
    ///
    /// Fails if the application has exited.
    pub fn close(&self, id: WindowId) -> std::result::Result<(), ClosedError> {
        self.tx
            .send(ProxyAction::Close(id))
            .map_err(|_| ClosedError)?;
        self.waker.send_event(()).map_err(|_| ClosedError)
    }

    /// Close all windows and terminate the UI.
    ///
    /// Fails if the application has exited.
    pub fn close_all(&self) -> std::result::Result<(), ClosedError> {
        self.tx
            .send(ProxyAction::CloseAll)
            .map_err(|_| ClosedError)?;
        self.waker.send_event(()).map_err(|_| ClosedError)
    }

    /// Send a message to [`AppData`] or a set recipient
    ///
    /// This is similar to [`EventCx::push`](crate::event::EventCx::push),
    /// but can only be handled by top-level [`AppData`] or a recipient set
    /// using [`ConfigCx::set_send_target_for`].
    ///
    /// Fails if the application has exited.
    pub fn push<M: std::fmt::Debug + Send + 'static>(
        &mut self,
        msg: M,
    ) -> std::result::Result<(), ClosedError> {
        self.tx
            .send(ProxyAction::Message(kas::messages::SendErased::new(msg)))
            .map_err(|_| ClosedError)?;
        self.waker.send_event(()).map_err(|_| ClosedError)
    }
}
