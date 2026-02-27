// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use super::{
    AppData, Error, GraphicsInstance, MessageStack, Pending, Platform, ProxyAction, RunError,
};
use crate::config::Config;
use crate::draw::{DrawShared, DrawSharedImpl, SharedState};
use crate::messages::Erased;
use crate::runner::GraphicsFeatures;
use crate::theme::Theme;
#[cfg(feature = "clipboard")]
use crate::util::warn_about_error;
use crate::window::{PopupDescriptor, Window as WindowWidget, WindowId, WindowIdFactory};
use crate::{ConfigAction, Id};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::mpsc;
use std::task::Waker;

#[cfg(feature = "clipboard")] use arboard::Clipboard;

/// Runner state shared by all windows and used by [`RunnerT`]
pub(super) struct Shared<Data: AppData, G: GraphicsInstance, T: Theme<G::Shared>> {
    pub(super) platform: Platform,
    config_writer: Option<Box<dyn FnMut(&Config)>>,
    pub(super) config: Rc<RefCell<Config>>,
    #[cfg(feature = "clipboard")]
    clipboard: Option<Clipboard>,
    pub(super) instance: G,
    pub(super) draw: Option<SharedState<G::Shared>>,
    pub(super) theme: T,
    pub(super) messages: MessageStack,
    pub(super) pending: VecDeque<Pending<Data>>,
    pub(super) send_queue: VecDeque<(Id, Erased)>,
    send_targets: HashMap<TypeId, Id>,
    pub(super) waker: Waker,
    pub(super) proxy_rx: mpsc::Receiver<ProxyAction>,
    window_id_factory: WindowIdFactory,
}

impl<A: AppData, G: GraphicsInstance, T: Theme<G::Shared>> Shared<A, G, T>
where
    T::Window: kas::theme::Window,
{
    /// Construct
    pub(super) fn new(
        platform: Platform,
        instance: G,
        theme: T,
        config: Rc<RefCell<Config>>,
        config_writer: Option<Box<dyn FnMut(&Config)>>,
        waker: Waker,
        proxy_rx: mpsc::Receiver<ProxyAction>,
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

        Ok(Shared {
            platform,
            config_writer,
            config,
            #[cfg(feature = "clipboard")]
            clipboard,
            instance,
            draw: None,
            theme,
            messages: MessageStack::new(),
            pending: Default::default(),
            send_queue: Default::default(),
            send_targets: Default::default(),
            waker,
            proxy_rx,
            window_id_factory,
        })
    }

    /// Redirect messages with a target defined by the type
    pub(crate) fn redirect_messages_by_type(&mut self) {
        if !self.messages.reset_and_has_any() {
            return;
        }

        let mut i = self.messages.stack.len();
        while i > 0 {
            i -= 1;
            if self.messages.stack[i].is_sent() {
                continue;
            }

            let type_id = self.messages.stack[i].type_id();
            if let Some(target) = self.send_targets.get(&type_id) {
                let msg = self.messages.stack.remove(i);
                self.send_queue.push_back((target.clone(), msg));
            }
        }
    }

    /// Flush pending messages
    pub(crate) fn handle_messages<Data: AppData>(&mut self, data: &mut Data) {
        if self.messages.reset_and_has_any() {
            let start_count = self.messages.get_op_count();
            let mut last_count = start_count;
            while !self.messages.stack.is_empty() {
                data.handle_message(&mut self.messages);
                if self.messages.get_op_count() == last_count {
                    break;
                } else {
                    last_count = self.messages.get_op_count();
                }
            }
            if self.messages.get_op_count() != start_count {
                self.pending.push_back(Pending::Update);
            }
        }

        self.messages.clear();
    }

    pub(crate) fn create_draw_shared(&mut self, surface: &G::Surface) -> Result<(), RunError> {
        if self.draw.is_none() {
            let features = GraphicsFeatures {
                subpixel_rendering: self
                    .config
                    .borrow()
                    .font
                    .raster()
                    .subpixel_mode
                    .any_subpixel(),
            };
            let mut draw_shared = self.instance.new_shared(Some(surface), features)?;
            draw_shared.set_raster_config(self.config.borrow().font.raster());
            self.draw = Some(SharedState::new(draw_shared));
        }

        Ok(())
    }

    pub(crate) fn suspended(&mut self) {
        if let Some(writer) = self.config_writer.as_mut() {
            self.config.borrow_mut().write_if_dirty(writer);
        }

        // NOTE: we assume that all windows are suspended when this is called
        self.draw = None;
    }
}

/// Runner shared-state type-erased interface
///
/// A `dyn RunnerT` object is used by [`crate::event::EventCx`].
pub(crate) trait RunnerT {
    /// Require configuration updates
    fn config_update(&mut self, action: ConfigAction);

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
    fn add_popup(&mut self, parent_id: WindowId, popup: PopupDescriptor) -> WindowId;

    /// Resize and reposition an existing pop-up
    fn reposition_popup(&mut self, id: WindowId, popup: PopupDescriptor);

    /// Add a window to the UI at run-time.
    fn add_dataless_window(&mut self, window: WindowWidget<()>) -> WindowId;

    /// Add a window to the UI at run-time.
    ///
    /// Safety: this method *should* require generic parameter `Data` (data type
    /// passed to the `Runner`). Realising this would require adding this type
    /// parameter to `EventCx` and thus to all widgets (not necessarily the
    /// type accepted by the widget as input). As an alternative we require the
    /// caller to type-cast `Window<Data>` to `Window<()>` and pass in
    /// `TypeId::of::<Data>()`.
    unsafe fn add_window(&mut self, window: WindowWidget<()>, data_type_id: TypeId) -> WindowId;

    /// Close a window
    fn close_window(&mut self, id: WindowId);

    /// Exit the application
    fn exit(&mut self);

    /// Access the message stack (read-only)
    fn message_stack(&self) -> &MessageStack;

    /// Access the message stack (mutable)
    fn message_stack_mut(&mut self) -> &mut MessageStack;

    /// Send a message to another window
    fn send_erased(&mut self, id: Id, msg: Erased);

    /// Set send targets
    fn set_send_targets(&mut self, targets: &mut Vec<(TypeId, Id)>);

    /// Find a send target for `type_id`, if any
    fn send_target_for(&self, type_id: TypeId) -> Option<Id>;

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

impl<Data: AppData, G: GraphicsInstance, T: Theme<G::Shared>> RunnerT for Shared<Data, G, T> {
    fn config_update(&mut self, action: ConfigAction) {
        self.pending.push_back(Pending::ConfigUpdate(action));
    }

    fn add_popup(&mut self, parent_id: WindowId, popup: PopupDescriptor) -> WindowId {
        let id = self.window_id_factory.make_next();
        self.pending
            .push_back(Pending::AddPopup(parent_id, id, popup));
        id
    }

    fn reposition_popup(&mut self, id: WindowId, popup: PopupDescriptor) {
        self.pending.push_back(Pending::RepositionPopup(id, popup));
    }

    fn add_dataless_window(&mut self, window: WindowWidget<()>) -> WindowId {
        let id = self.window_id_factory.make_next();
        self.pending
            .push_back(Pending::AddWindow(id, window.map_any().boxed()));
        id
    }

    unsafe fn add_window(&mut self, window: WindowWidget<()>, data_type_id: TypeId) -> WindowId {
        // Safety: the window should be `Window<Data>`. We cast to that.
        if data_type_id != TypeId::of::<Data>() {
            // If this fails it is not safe to add the window (though we could just return).
            panic!("add_window: window has wrong Data type!");
        }
        let window: WindowWidget<Data> = unsafe { std::mem::transmute(window) };

        // By far the simplest way to implement this is to let our call
        // anscestor, event::Loop::handle, do the work.
        //
        // In theory we could pass the `ActiveEventLoop` for *each* event
        // handled to create the winit window here or use statics to generate
        // errors now, but user code can't do much with this error anyway.
        let id = self.window_id_factory.make_next();
        self.pending
            .push_back(Pending::AddWindow(id, window.boxed()));
        id
    }

    fn close_window(&mut self, id: WindowId) {
        self.pending.push_back(Pending::CloseWindow(id));
    }

    fn exit(&mut self) {
        self.pending.push_back(Pending::Exit);
    }

    fn message_stack(&self) -> &MessageStack {
        &self.messages
    }

    fn message_stack_mut(&mut self) -> &mut MessageStack {
        &mut self.messages
    }

    fn send_erased(&mut self, id: Id, msg: Erased) {
        self.send_queue.push_back((id, msg));
    }

    /// Set send targets
    fn set_send_targets(&mut self, targets: &mut Vec<(TypeId, Id)>) {
        for (type_id, id) in targets.drain(..) {
            self.send_targets.insert(type_id, id);
        }
    }

    /// Find a send target for `type_id`, if any
    fn send_target_for(&self, type_id: TypeId) -> Option<Id> {
        self.send_targets.get(&type_id).cloned()
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
