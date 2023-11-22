// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Public shell stuff common to all backends

use crate::draw::DrawSharedImpl;
use crate::draw::{color::Rgba, DrawIface, WindowCommon};
use crate::geom::Size;
use crate::theme::Theme;
use raw_window_handle as raw;
use std::time::Instant;
use thiserror::Error;

/// Possible failures from constructing a [`Shell`](super::Shell)
///
/// Some variants are undocumented. Users should not match these variants since
/// they are not considered part of the public API.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
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

/// API for the graphical implementation of a shell
///
/// See also [`Shell`](super::Shell).
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait GraphicalShell {
    /// The default theme
    type DefaultTheme: Default + Theme<Self::Shared>;

    /// Shared draw state
    type Shared: DrawSharedImpl;

    /// Window surface
    type Surface: WindowSurface<Shared = Self::Shared> + 'static;

    /// Construct shared state
    fn build(self) -> Result<Self::Shared>;
}

/// Window graphical surface requirements
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait WindowSurface {
    /// Shared draw state
    type Shared: kas::draw::DrawSharedImpl;

    /// Construct an instance from a window handle
    ///
    /// It is required to call [`WindowSurface::do_resize`] after this.
    fn new<W>(shared: &mut Self::Shared, window: W) -> Result<Self>
    where
        W: raw::HasRawWindowHandle + raw::HasRawDisplayHandle,
        Self: Sized;

    /// Get current surface size
    fn size(&self) -> Size;

    /// Resize surface
    ///
    /// Returns `true` when the new `size` did not match the old surface size.
    fn do_resize(&mut self, shared: &mut Self::Shared, size: Size) -> bool;

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
