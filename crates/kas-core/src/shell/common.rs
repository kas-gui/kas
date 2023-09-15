// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Public shell stuff common to all backends

use crate::draw::{color::Rgba, DrawIface, WindowCommon};
use crate::draw::{DrawImpl, DrawShared, DrawSharedImpl};
use crate::event::CursorIcon;
use crate::geom::Size;
use crate::theme::{ThemeControl, ThemeSize};
use crate::{Action, Window, WindowId};
use raw_window_handle as raw;
use std::any::TypeId;
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
    /// Shared draw state
    type Shared: DrawSharedImpl;

    /// Per-window draw state
    type Window: DrawImpl;

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
    fn new<W: raw::HasRawWindowHandle + raw::HasRawDisplayHandle>(
        shared: &mut Self::Shared,
        size: Size,
        window: W,
    ) -> Result<Self>
    where
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
    fn present(&mut self, shared: &mut Self::Shared, clear_color: Rgba);
}

/// Window management interface
///
/// Note: previously, this was implemented by a dependent crate. Now, it is not,
/// which might suggest this trait is no longer needed, however `EventCx` still
/// needs type erasure over `S: WindowSurface` and `T: Theme`.
pub(crate) trait ShellWindow {
    /// Add a pop-up
    ///
    /// A pop-up may be presented as an overlay layer in the current window or
    /// via a new borderless window.
    ///
    /// Pop-ups support position hints: they are placed *next to* the specified
    /// `rect`, preferably in the given `direction`.
    ///
    /// Returns `None` if window creation is not currently available (but note
    /// that `Some` result does not guarantee the operation succeeded).
    fn add_popup(&mut self, popup: crate::PopupDescriptor) -> WindowId;

    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added from an
    /// event handler, albeit without error handling.
    ///
    /// Safety: this method *should* require generic parameter `Data` (data type
    /// passed to the `Shell`). Realising this would require adding this type
    /// parameter to `EventCx` and thus to all widgets (not necessarily the
    /// type accepted by the widget as input). As an alternative we require the
    /// caller to type-cast `Window<Data>` to `Window<()>` and pass in
    /// `TypeId::of::<Data>()`.
    unsafe fn add_window(&mut self, window: Window<()>, data_type_id: TypeId) -> WindowId;

    /// Close a window
    fn close_window(&mut self, id: WindowId);

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    fn get_clipboard(&mut self) -> Option<String>;

    /// Attempt to set clipboard contents
    fn set_clipboard(&mut self, content: String);

    /// Get contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    fn get_primary(&mut self) -> Option<String>;

    /// Set contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    fn set_primary(&mut self, content: String);

    /// Adjust the theme
    ///
    /// Note: theme adjustments apply to all windows, as does the [`Action`]
    /// returned from the closure.
    //
    // TODO(opt): pass f by value, not boxed
    fn adjust_theme<'s>(&'s mut self, f: Box<dyn FnOnce(&mut dyn ThemeControl) -> Action + 's>);

    /// Access the [`ThemeSize`] object
    fn theme_size(&self) -> &dyn ThemeSize;

    /// Access the [`DrawShared`] object
    fn draw_shared(&mut self) -> &mut dyn DrawShared;

    /// Set the mouse cursor
    fn set_cursor_icon(&mut self, icon: CursorIcon);

    /// Directly access Winit Window
    ///
    /// This is a temporary API, allowing e.g. to minimize the window.
    #[cfg(winit)]
    fn winit_window(&self) -> Option<&winit::window::Window>;

    /// Access a Waker
    fn waker(&self) -> &std::task::Waker;
}
