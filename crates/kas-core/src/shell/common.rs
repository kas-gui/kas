// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Public shell stuff common to all backends

use crate::draw::{color::Rgba, DrawIface, WindowCommon};
use crate::draw::{DrawImpl, DrawShared, DrawSharedImpl};
use crate::event::{CursorIcon, UpdateId};
use crate::geom::Size;
use crate::theme::{RasterConfig, ThemeControl, ThemeSize};
use crate::{Action, WindowId};
use raw_window_handle as raw;
use thiserror::Error;
#[cfg(feature = "winit")] use winit::error::OsError;

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
    #[doc(hidden)]

    /// OS error during window creation
    #[error("operating system error")]
    #[cfg(feature = "winit")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "winit")))]
    Window(#[from] OsError),
}

/// A `Result` type representing `T` or [`enum@Error`]
pub type Result<T> = std::result::Result<T, Error>;

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
    fn build(self, raster_config: &RasterConfig) -> Result<Self::Shared>;
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
    ) -> Self;

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
/// which might suggest this trait is no longer needed, however `EventMgr` still
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
    fn add_popup(&mut self, popup: crate::Popup) -> Option<WindowId>;

    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added from an
    /// event handler, albeit without error handling.
    fn add_window(&mut self, widget: Box<dyn crate::Window>) -> WindowId;

    /// Close a window
    fn close_window(&mut self, id: WindowId);

    /// Updates all subscribed widgets
    ///
    /// All widgets subscribed to the given [`UpdateId`], across all
    /// windows, will receive an update.
    fn update_all(&mut self, id: UpdateId, payload: u64);

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    fn get_clipboard(&mut self) -> Option<String>;

    /// Attempt to set clipboard contents
    fn set_clipboard(&mut self, content: String);

    /// Adjust the theme
    ///
    /// Note: theme adjustments apply to all windows, as does the [`Action`]
    /// returned from the closure.
    fn adjust_theme(&mut self, f: &mut dyn FnMut(&mut dyn ThemeControl) -> Action);

    /// Access [`ThemeSize`] and [`DrawShared`] objects
    ///
    /// Implementations should call the given function argument once; not doing
    /// so is memory-safe but will cause panics in `EventMgr` methods.
    /// User-code *must not* depend on `f` being called for memory safety.
    fn size_and_draw_shared(&mut self, f: &mut dyn FnMut(&mut dyn ThemeSize, &mut dyn DrawShared));

    /// Set the mouse cursor
    fn set_cursor_icon(&mut self, icon: CursorIcon);

    /// Access a Waker
    fn waker(&self) -> &std::task::Waker;
}
