// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use log::{info, warn};
use std::num::NonZeroU32;

use crate::draw::ShaderManager;
use crate::{Error, Options, WindowId};
use kas::event::UpdateHandle;

#[cfg(feature = "clipboard")]
use clipboard::{ClipboardContext, ClipboardProvider};

/// State shared between windows
pub struct SharedState<T> {
    #[cfg(feature = "clipboard")]
    clipboard: Option<ClipboardContext>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub shaders: ShaderManager,
    pub theme: T,
    pub pending: Vec<PendingAction>,
    window_id: u32,
}

impl<T> SharedState<T> {
    /// Construct
    pub fn new(theme: T, options: Options) -> Result<Self, Error> {
        #[cfg(feature = "clipboard")]
        let clipboard = match ClipboardContext::new() {
            Ok(cb) => Some(cb),
            Err(e) => {
                warn!("Unable to open clipboard: {:?}", e);
                None
            }
        };

        let adapter_options = options.adapter_options();

        let adapter = match wgpu::Adapter::request(&adapter_options) {
            Some(a) => a,
            None => return Err(Error::NoAdapter),
        };
        info!("Using graphics adapter: {}", adapter.get_info().name);

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let shaders = ShaderManager::new(&device)?;

        Ok(SharedState {
            #[cfg(feature = "clipboard")]
            clipboard,
            device,
            queue,
            shaders,
            theme,
            pending: vec![],
            window_id: 0,
        })
    }

    pub fn next_window_id(&mut self) -> WindowId {
        self.window_id += 1;
        WindowId::new(NonZeroU32::new(self.window_id).unwrap())
    }

    #[cfg(not(feature = "clipboard"))]
    #[inline]
    pub fn get_clipboard(&mut self) -> Option<String> {
        None
    }

    #[cfg(feature = "clipboard")]
    pub fn get_clipboard(&mut self) -> Option<String> {
        self.clipboard
            .as_mut()
            .and_then(|cb| match cb.get_contents() {
                Ok(c) => Some(c),
                Err(e) => {
                    warn!("Failed to get clipboard contents: {:?}", e);
                    None
                }
            })
    }

    #[cfg(not(feature = "clipboard"))]
    #[inline]
    pub fn set_clipboard(&mut self, _content: String) {}

    #[cfg(feature = "clipboard")]
    pub fn set_clipboard(&mut self, content: String) {
        self.clipboard.as_mut().map(|cb| {
            cb.set_contents(content)
                .unwrap_or_else(|e| warn!("Failed to set clipboard contents: {:?}", e))
        });
    }
}

// For some implementations this may need to be implemented on a
// window-specific handle; for us this is unnecessary.
//
// NOTE: Correct clipboard handling on Wayland requires a window handle.
impl<T> kas::TkWindow for SharedState<T> {
    fn add_window(&mut self, widget: Box<dyn kas::Window>) -> WindowId {
        // By far the simplest way to implement this is to let our call
        // anscestor, event::Loop::handle, do the work.
        //
        // In theory we could pass the EventLoopWindowTarget for *each* event
        // handled to create the winit window here or use statics to generate
        // errors now, but user code can't do much with this error anyway.
        let id = self.next_window_id();
        self.pending.push(PendingAction::AddWindow(id, widget));
        id
    }

    fn close_window(&mut self, id: WindowId) {
        self.pending.push(PendingAction::CloseWindow(id));
    }

    fn trigger_update(&mut self, handle: UpdateHandle, payload: u64) {
        self.pending.push(PendingAction::Update(handle, payload));
    }

    #[inline]
    fn get_clipboard(&mut self) -> Option<String> {
        self.get_clipboard()
    }

    #[inline]
    fn set_clipboard(&mut self, content: String) {
        self.set_clipboard(content);
    }
}

pub enum PendingAction {
    AddWindow(WindowId, Box<dyn kas::Window>),
    CloseWindow(WindowId),
    Update(UpdateHandle, u64),
}
