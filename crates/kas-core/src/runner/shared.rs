// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use super::{AppData, Error, GraphicsInstance, Pending, Platform};
use crate::config::Config;
use crate::draw::{DrawShared, DrawSharedImpl};
use crate::theme::Theme;
use crate::util::warn_about_error;
use crate::WindowIdFactory;
use crate::{draw, messages::MessageStack, Action, WindowId};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::task::Waker;

#[cfg(feature = "clipboard")] use arboard::Clipboard;

/// Runner state used by [`RunnerT`]
pub(super) struct SharedState<Data: AppData, G: GraphicsInstance, T: Theme<G::Shared>> {
    pub(super) platform: Platform,
    pub(super) config: Rc<RefCell<Config>>,
    #[cfg(feature = "clipboard")]
    clipboard: Option<Clipboard>,
    pub(super) draw: Option<draw::SharedState<G::Shared>>,
    pub(super) theme: T,
    pub(super) pending: VecDeque<Pending<Data, G, T>>,
    pub(super) waker: Waker,
    window_id_factory: WindowIdFactory,
}

/// Runner state shared by all windows
pub(super) struct State<Data: AppData, G: GraphicsInstance, T: Theme<G::Shared>> {
    pub(super) instance: G,
    pub(super) shared: SharedState<Data, G, T>,
    pub(super) data: Data,
    config_writer: Option<Box<dyn FnMut(&Config)>>,
}

impl<Data: AppData, G: GraphicsInstance, T: Theme<G::Shared>> State<Data, G, T>
where
    T::Window: kas::theme::Window,
{
    /// Construct
    pub(super) fn new(
        platform: Platform,
        data: Data,
        instance: G,
        theme: T,
        config: Rc<RefCell<Config>>,
        config_writer: Option<Box<dyn FnMut(&Config)>>,
        waker: Waker,
        window_id_factory: WindowIdFactory,
    ) -> Result<Self, Error> {
        #[cfg(feature = "clipboard")]
        let clipboard = match Clipboard::new() {
            Ok(cb) => Some(cb),
            Err(e) => {
                warn_about_error("Failed to connect clipboard", &e);
                None
            }
        };

        Ok(State {
            instance,
            shared: SharedState {
                platform,
                config,
                #[cfg(feature = "clipboard")]
                clipboard,
                draw: None,
                theme,
                pending: Default::default(),
                waker,
                window_id_factory,
            },
            data,
            config_writer,
        })
    }

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

    pub(crate) fn resume(&mut self, surface: &G::Surface<'_>) -> Result<(), Error> {
        if self.shared.draw.is_none() {
            let mut draw_shared = self.instance.new_shared(Some(surface))?;
            draw_shared.set_raster_config(self.shared.config.borrow().font.raster());
            self.shared.draw = Some(kas::draw::SharedState::new(draw_shared));
        }

        Ok(())
    }

    pub(crate) fn suspended(&mut self) {
        self.data.suspended();

        if let Some(writer) = self.config_writer.as_mut() {
            self.shared.config.borrow_mut().write_if_dirty(writer);
        }

        // NOTE: we assume that all windows are suspended when this is called
        self.shared.draw = None;
    }
}

/// Runner shared-state type-erased interface
///
/// A `dyn RunnerT` object is used by [`crate::event::EventCx`].
pub(crate) trait RunnerT {
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
    /// passed to the `Runner`). Realising this would require adding this type
    /// parameter to `EventCx` and thus to all widgets (not necessarily the
    /// type accepted by the widget as input). As an alternative we require the
    /// caller to type-cast `Window<Data>` to `Window<()>` and pass in
    /// `TypeId::of::<Data>()`.
    unsafe fn add_window(&mut self, window: kas::Window<()>, data_type_id: TypeId) -> WindowId;

    /// Close a window
    fn close_window(&mut self, id: WindowId);

    /// Exit the application
    fn exit(&mut self);

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

    /// Access the [`DrawShared`] object
    fn draw_shared(&mut self) -> &mut dyn DrawShared;

    /// Access a Waker
    fn waker(&self) -> &std::task::Waker;
}

impl<Data: AppData, G: GraphicsInstance, T: Theme<G::Shared>> RunnerT for SharedState<Data, G, T> {
    fn add_popup(&mut self, parent_id: WindowId, popup: kas::PopupDescriptor) -> WindowId {
        let id = self.window_id_factory.make_next();
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
        // In theory we could pass the `ActiveEventLoop` for *each* event
        // handled to create the winit window here or use statics to generate
        // errors now, but user code can't do much with this error anyway.
        let id = self.window_id_factory.make_next();
        let window = Box::new(super::Window::new(
            self.config.clone(),
            self.platform,
            id,
            window,
        ));
        self.pending.push_back(Pending::AddWindow(id, window));
        id
    }

    fn close_window(&mut self, id: WindowId) {
        self.pending.push_back(Pending::CloseWindow(id));
    }

    fn exit(&mut self) {
        self.pending.push_back(Pending::Exit);
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

    fn draw_shared(&mut self) -> &mut dyn DrawShared {
        // We can expect draw to be initialized from any context where this trait is used
        self.draw.as_mut().unwrap()
    }

    #[inline]
    fn waker(&self) -> &std::task::Waker {
        &self.waker
    }
}
