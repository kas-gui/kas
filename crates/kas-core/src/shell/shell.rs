// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`Shell`] and supporting elements

use super::{GraphicalShell, Platform, ProxyAction, Result, SharedState};
use crate::config::Options;
use crate::draw::{DrawImpl, DrawShared, DrawSharedImpl};
use crate::event::{self, UpdateId};
use crate::model::SharedRc;
use crate::theme::{self, Theme, ThemeConfig};
use crate::util::warn_about_error;
use crate::{Window, WindowId};
use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopProxy};

/// The KAS shell
///
/// The "shell" is the layer over widgets, windows, events and graphics.
///
/// Constructing with [`Shell::new`] or [`Shell::new_custom`]
/// reads configuration (depending on passed options or environment variables)
/// and initialises the font database. Note that this database is a global
/// singleton and some widgets and other library code may expect fonts to have
/// been initialised first.
pub struct Shell<G: GraphicalShell, T: Theme<G::Shared>> {
    el: EventLoop<ProxyAction>,
    windows: Vec<super::Window<G::Surface, T>>,
    shared: SharedState<G::Surface, T>,
}

/// Shell associated types
///
/// Note: these could be inherent associated types of [`Shell`] when Rust#8995 is stable.
pub trait ShellAssoc {
    /// Shared draw state type
    type DrawShared: DrawSharedImpl;

    /// Per-window draw state
    type Draw: DrawImpl;
}

impl<G: GraphicalShell, T: Theme<G::Shared> + 'static> ShellAssoc for Shell<G, T>
where
    T::Window: theme::Window,
{
    type DrawShared = G::Shared;

    type Draw = G::Window;
}

impl<G: GraphicalShell + Default, T: Theme<G::Shared> + 'static> Shell<G, T>
where
    T::Window: theme::Window,
{
    /// Construct a new instance with default options.
    ///
    /// Environment variables may affect option selection; see documentation
    /// of [`Options::from_env`]. KAS config is provided by
    /// [`Options::read_config`].
    #[inline]
    pub fn new(theme: T) -> Result<Self> {
        Self::new_custom(G::default(), theme, Options::from_env())
    }
}

impl<G: GraphicalShell, T: Theme<G::Shared> + 'static> Shell<G, T>
where
    T::Window: theme::Window,
{
    /// Construct an instance with custom options
    ///
    /// The [`Options`] parameter allows direct specification of shell options;
    /// usually, these are provided by [`Options::from_env`].
    ///
    /// KAS config is provided by [`Options::read_config`] and `theme` is
    /// configured through [`Options::init_theme_config`].
    #[inline]
    pub fn new_custom(
        graphical_shell: impl Into<G>,
        mut theme: T,
        options: Options,
    ) -> Result<Self> {
        options.init_theme_config(&mut theme)?;
        let config = match options.read_config() {
            Ok(config) => config,
            Err(error) => {
                warn_about_error("Shell::new_custom: failed to read config", &error);
                Default::default()
            }
        };
        let config = SharedRc::new(config);

        Self::new_custom_config(graphical_shell, theme, options, config)
    }

    /// Construct an instance with custom options and config
    ///
    /// This is like [`Shell::new_custom`], but allows KAS config to be
    /// specified directly, instead of loading via [`Options::read_config`].
    ///
    /// Unlike other the constructors, this method does not configure the theme.
    /// The user should call [`Options::init_theme_config`] before this method.
    #[inline]
    pub fn new_custom_config(
        graphical_shell: impl Into<G>,
        theme: T,
        options: Options,
        config: SharedRc<event::Config>,
    ) -> Result<Self> {
        let el = EventLoopBuilder::with_user_event().build();
        let windows = vec![];

        let mut draw_shared = graphical_shell.into().build()?;
        draw_shared.set_raster_config(theme.config().raster());
        let pw = PlatformWrapper(&el);
        let shared = SharedState::new(pw, draw_shared, theme, options, config)?;

        Ok(Shell {
            el,
            windows,
            shared,
        })
    }

    /// Access shared draw state
    #[inline]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        &mut self.shared.draw
    }

    /// Access event configuration
    #[inline]
    pub fn event_config(&self) -> &SharedRc<event::Config> {
        &self.shared.config
    }

    /// Access the theme by ref
    #[inline]
    pub fn theme(&self) -> &T {
        &self.shared.theme
    }

    /// Access the theme by ref mut
    #[inline]
    pub fn theme_mut(&mut self) -> &mut T {
        &mut self.shared.theme
    }

    /// Assume ownership of and display a window
    #[inline]
    pub fn add(&mut self, window: Window) -> Result<WindowId> {
        let id = self.shared.next_window_id();
        let win = super::Window::new(&mut self.shared, &self.el, id, window)?;
        self.windows.push(win);
        Ok(id)
    }

    /// Assume ownership of and display a window, inline
    #[inline]
    pub fn with(mut self, window: Window) -> Result<Self> {
        let _ = self.add(window)?;
        Ok(self)
    }

    /// Create a proxy which can be used to update the UI from another thread
    pub fn create_proxy(&self) -> Proxy {
        Proxy(self.el.create_proxy())
    }

    /// Run the main loop.
    #[inline]
    pub fn run(self) -> ! {
        let mut el = super::EventLoop::new(self.windows, self.shared);
        self.el
            .run(move |event, elwt, control_flow| el.handle(event, elwt, control_flow))
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

    /// Trigger an update
    pub fn update_all(&self, id: UpdateId, payload: u64) -> std::result::Result<(), ClosedError> {
        self.0
            .send_event(ProxyAction::Update(id, payload))
            .map_err(|_| ClosedError)
    }

    /// Wake async methods
    fn wake_async(&self) {
        // ignore error: if the loop closed the future has been dropped
        let _ = self.0.send_event(ProxyAction::WakeAsync);
    }
}
