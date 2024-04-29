// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use super::{AppData, AppGraphicsBuilder, Error, Pending, Platform};
use crate::config::{Config, Options};
use crate::draw::DrawShared;
use crate::theme::{Theme, ThemeControl};
use crate::util::warn_about_error;
use crate::{draw, messages::MessageStack, Action, WindowId};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::task::Waker;

#[cfg(feature = "clipboard")] use arboard::Clipboard;

/// Application state used by [`AppShared`]
pub(crate) struct AppSharedState<Data: AppData, G: AppGraphicsBuilder, T: Theme<G::Shared>> {
    pub(super) platform: Platform,
    pub(super) config: Rc<RefCell<Config>>,
    #[cfg(feature = "clipboard")]
    clipboard: Option<Clipboard>,
    pub(super) draw: draw::SharedState<G::Shared>,
    pub(super) theme: T,
    pub(super) pending: VecDeque<Pending<Data, G, T>>,
    pub(super) waker: Waker,
    window_id: u32,
}

/// Application state shared by all windows
pub(crate) struct AppState<Data: AppData, G: AppGraphicsBuilder, T: Theme<G::Shared>> {
    pub(super) shared: AppSharedState<Data, G, T>,
    pub(super) data: Data,
    /// Estimated scale factor (from last window constructed or available screens)
    options: Options,
}

impl<Data: AppData, G: AppGraphicsBuilder, T: Theme<G::Shared>> AppState<Data, G, T>
where
    T::Window: kas::theme::Window,
{
    /// Construct
    pub(super) fn new(
        data: Data,
        pw: super::PlatformWrapper,
        draw_shared: G::Shared,
        mut theme: T,
        options: Options,
        config: Rc<RefCell<Config>>,
    ) -> Result<Self, Error> {
        let platform = pw.platform();
        let mut draw = kas::draw::SharedState::new(draw_shared);
        theme.init(&mut draw);

        #[cfg(feature = "clipboard")]
        let clipboard = match Clipboard::new() {
            Ok(cb) => Some(cb),
            Err(e) => {
                warn_about_error("Failed to connect clipboard", &e);
                None
            }
        };

        Ok(AppState {
            shared: AppSharedState {
                platform,
                config,
                #[cfg(feature = "clipboard")]
                clipboard,
                draw,
                theme,
                pending: Default::default(),
                waker: pw.create_waker(),
                window_id: 0,
            },
            data,
            options,
        })
    }

    #[inline]
    pub(crate) fn handle_messages(&mut self, messages: &mut MessageStack) {
        if messages.reset_and_has_any() {
            let count = messages.get_op_count();
            self.data.handle_messages(messages);
            if messages.get_op_count() != count {
                self.shared
                    .pending
                    .push_back(Pending::Action(Action::UPDATE));
            }
        }
    }

    pub(crate) fn on_exit(&self) {
        match self
            .options
            .write_config(&self.shared.config.borrow(), &self.shared.theme)
        {
            Ok(()) => (),
            Err(error) => warn_about_error("Failed to save config", &error),
        }
    }
}

impl<Data: AppData, G: AppGraphicsBuilder, T: Theme<G::Shared>> AppSharedState<Data, G, T> {
    /// Return the next window identifier
    ///
    /// TODO(opt): this should recycle used identifiers since Id does not
    /// efficiently represent large numbers.
    pub(crate) fn next_window_id(&mut self) -> WindowId {
        let id = self.window_id + 1;
        self.window_id = id;
        WindowId::new(NonZeroU32::new(id).unwrap())
    }
}

/// Application shared-state type-erased interface
///
/// A `dyn AppShared` object is used by [crate::event::`EventCx`].
pub(crate) trait AppShared {
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
    fn add_popup(&mut self, parent_id: WindowId, popup: crate::PopupDescriptor) -> WindowId;

    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added from an
    /// event handler, albeit without error handling.
    ///
    /// Safety: this method *should* require generic parameter `Data` (data type
    /// passed to the `Application`). Realising this would require adding this type
    /// parameter to `EventCx` and thus to all widgets (not necessarily the
    /// type accepted by the widget as input). As an alternative we require the
    /// caller to type-cast `Window<Data>` to `Window<()>` and pass in
    /// `TypeId::of::<Data>()`.
    unsafe fn add_window(&mut self, window: kas::Window<()>, data_type_id: TypeId) -> WindowId;

    /// Close a window
    fn close_window(&mut self, id: WindowId);

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    ///
    /// NOTE: on Wayland, use `WindowDataErased::wayland_clipboard` instead.
    /// This split API probably can't be resolved until Winit integrates
    /// clipboard support.
    fn get_clipboard(&mut self) -> Option<String>;

    /// Attempt to set clipboard contents
    ///
    /// NOTE: on Wayland, use `WindowDataErased::wayland_clipboard` instead.
    /// This split API probably can't be resolved until Winit integrates
    /// clipboard support.
    fn set_clipboard(&mut self, content: String);

    /// Get contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    ///
    /// NOTE: on Wayland, use `WindowDataErased::wayland_clipboard` instead.
    /// This split API probably can't be resolved until Winit integrates
    /// clipboard support.
    fn get_primary(&mut self) -> Option<String>;

    /// Set contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    ///
    /// NOTE: on Wayland, use `WindowDataErased::wayland_clipboard` instead.
    /// This split API probably can't be resolved until Winit integrates
    /// clipboard support.
    fn set_primary(&mut self, content: String);

    /// Adjust the theme
    ///
    /// Note: theme adjustments apply to all windows, as does the [`Action`]
    /// returned from the closure.
    //
    // TODO(opt): pass f by value, not boxed
    fn adjust_theme<'s>(&'s mut self, f: Box<dyn FnOnce(&mut dyn ThemeControl) -> Action + 's>);

    /// Access the [`DrawShared`] object
    fn draw_shared(&mut self) -> &mut dyn DrawShared;

    /// Access a Waker
    fn waker(&self) -> &std::task::Waker;
}

impl<Data: AppData, G: AppGraphicsBuilder, T: Theme<G::Shared>> AppShared
    for AppSharedState<Data, G, T>
{
    fn add_popup(&mut self, parent_id: WindowId, popup: kas::PopupDescriptor) -> WindowId {
        let id = self.next_window_id();
        self.pending
            .push_back(Pending::AddPopup(parent_id, id, popup));
        id
    }

    unsafe fn add_window(&mut self, window: kas::Window<()>, data_type_id: TypeId) -> WindowId {
        // Safety: the window should be `Window<Data>`. We cast to that.
        if data_type_id != TypeId::of::<Data>() {
            // If this fails it is not safe to add the window (though we could just return).
            panic!("add_window: window has wrong Data type!");
        }
        let window: kas::Window<Data> = std::mem::transmute(window);

        // By far the simplest way to implement this is to let our call
        // anscestor, event::Loop::handle, do the work.
        //
        // In theory we could pass the EventLoopWindowTarget for *each* event
        // handled to create the winit window here or use statics to generate
        // errors now, but user code can't do much with this error anyway.
        let id = self.next_window_id();
        let window = Box::new(super::Window::new(self, id, window));
        self.pending.push_back(Pending::AddWindow(id, window));
        id
    }

    fn close_window(&mut self, id: WindowId) {
        self.pending.push_back(Pending::CloseWindow(id));
    }

    fn get_clipboard(&mut self) -> Option<String> {
        #[cfg(feature = "clipboard")]
        {
            if let Some(cb) = self.clipboard.as_mut() {
                match cb.get_text() {
                    Ok(s) => return Some(s),
                    Err(e) => warn_about_error("Failed to get clipboard contents", &e),
                }
            }
        }

        None
    }

    fn set_clipboard<'c>(&mut self, _content: String) {
        #[cfg(feature = "clipboard")]
        if let Some(cb) = self.clipboard.as_mut() {
            match cb.set_text(_content) {
                Ok(()) => (),
                Err(e) => warn_about_error("Failed to set clipboard contents", &e),
            }
        }
    }

    fn get_primary(&mut self) -> Option<String> {
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten")),
            feature = "clipboard",
        ))]
        {
            use arboard::{GetExtLinux, LinuxClipboardKind};
            if let Some(cb) = self.clipboard.as_mut() {
                match cb.get().clipboard(LinuxClipboardKind::Primary).text() {
                    Ok(s) => return Some(s),
                    Err(e) => warn_about_error("Failed to get clipboard contents", &e),
                }
            }
        }

        None
    }

    fn set_primary(&mut self, _content: String) {
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten")),
            feature = "clipboard",
        ))]
        if let Some(cb) = self.clipboard.as_mut() {
            use arboard::{LinuxClipboardKind, SetExtLinux};
            match cb
                .set()
                .clipboard(LinuxClipboardKind::Primary)
                .text(_content)
            {
                Ok(()) => (),
                Err(e) => warn_about_error("Failed to set clipboard contents", &e),
            }
        }
    }

    fn adjust_theme<'s>(&'s mut self, f: Box<dyn FnOnce(&mut dyn ThemeControl) -> Action + 's>) {
        let action = f(&mut self.theme);
        self.pending.push_back(Pending::Action(action));
    }

    fn draw_shared(&mut self) -> &mut dyn DrawShared {
        &mut self.draw
    }

    #[inline]
    fn waker(&self) -> &std::task::Waker {
        &self.waker
    }
}
