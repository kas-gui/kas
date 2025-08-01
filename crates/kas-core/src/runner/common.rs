// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Public items common to all backends

use crate::draw::DrawSharedImpl;
use crate::draw::{DrawIface, WindowCommon, color::Rgba};
use crate::geom::Size;
use raw_window_handle as rwh;
use std::time::Instant;
use thiserror::Error;

/// Possible launch failures
///
/// Some variants are undocumented. Users should not match these variants since
/// they are not considered part of the public API.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    /// Window-handle error
    #[error(transparent)]
    Handle(#[from] rwh::HandleError),

    /// Failure from the graphics sub-system
    #[error("error from graphics sub-system")]
    Graphics(Box<dyn std::error::Error + 'static>),

    /// Config load/save error
    #[error("config load/save error")]
    Config(#[from] kas::config::Error),

    /// Event loop error
    #[error("event loop")]
    EventLoop(#[from] winit::error::EventLoopError),
}

impl From<winit::error::OsError> for Error {
    fn from(error: winit::error::OsError) -> Self {
        Error::EventLoop(winit::error::EventLoopError::Os(error))
    }
}

/// A `Result` type representing `T` or [`enum@Error`]
pub type Result<T> = std::result::Result<T, Error>;

/// Enumeration of platforms
///
/// Each option is compile-time enabled only if that platform is possible.
/// Methods like [`Self::is_wayland`] are available on all platforms.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Platform {
    #[cfg(target_os = "android")]
    Android,
    #[cfg(target_os = "ios")]
    IOS,
    #[cfg(target_os = "macos")]
    MacOS,
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    Wayland,
    #[cfg(target_arch = "wasm32")]
    Web,
    #[cfg(target_os = "windows")]
    Windows,
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    X11,
}

impl Platform {
    /// True if the platform is Android
    pub fn is_android(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(target_os = "android")] {
                true
            } else {
                false
            }
        }
    }

    /// True if the platform is IOS
    pub fn is_ios(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(target_os = "ios")] {
                true
            } else {
                false
            }
        }
    }

    /// True if the platform is MacOS
    pub fn is_macos(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(target_os = "macos")] {
                true
            } else {
                false
            }
        }
    }

    /// True if the platform is Wayland
    pub fn is_wayland(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))] {
                matches!(self, Platform::Wayland)
            } else {
                false
            }
        }
    }

    /// True if the platform is Web
    pub fn is_web(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                true
            } else {
                false
            }
        }
    }

    /// True if the platform is Windows
    pub fn is_windows(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(target_os = "windows")] {
                true
            } else {
                false
            }
        }
    }

    /// True if the platform is X11
    pub fn is_x11(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))] {
                matches!(self, Platform::X11)
            } else {
                false
            }
        }
    }
}

/// Context for a graphics backend
pub trait GraphicsInstance {
    /// Draw state shared by all windows
    type Shared: DrawSharedImpl;

    /// Window surface
    type Surface<'a>: WindowSurface<Shared = Self::Shared>;

    /// Construct shared state
    ///
    /// Providing a `surface` may aid construction of a graphics adapter
    /// (see [`compatible_surface`](https://docs.rs/wgpu/latest/wgpu/type.RequestAdapterOptions.html#structfield.compatible_surface)).
    fn new_shared(&mut self, surface: Option<&Self::Surface<'_>>) -> Result<Self::Shared>;

    /// Construct a window surface
    ///
    /// It is required to call [`WindowSurface::configure`] after this.
    fn new_surface<'window, W>(
        &mut self,
        window: W,
        transparent: bool,
    ) -> Result<Self::Surface<'window>>
    where
        W: rwh::HasWindowHandle + rwh::HasDisplayHandle + Send + Sync + 'window,
        Self: Sized;
}

/// Window graphical surface requirements
pub trait WindowSurface {
    /// Shared draw state
    type Shared: kas::draw::DrawSharedImpl;

    /// Get current surface size
    fn size(&self) -> Size;

    /// Resize surface
    ///
    /// Returns `true` when the new `size` did not match the old surface size.
    fn configure(&mut self, shared: &mut Self::Shared, size: Size) -> bool;

    /// Construct a DrawIface object
    fn draw_iface<'iface>(
        &'iface mut self,
        shared: &'iface mut kas::draw::SharedState<Self::Shared>,
    ) -> DrawIface<'iface, Self::Shared>;

    /// Access common data
    fn common_mut(&mut self) -> &mut WindowCommon;

    /// Present frame
    ///
    /// Return time at which render finishes
    fn present(&mut self, shared: &mut Self::Shared, clear_color: Rgba) -> Instant;
}
