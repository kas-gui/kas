// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use log::{info, warn};

use crate::Error;

#[cfg(feature = "clipboard")]
use clipboard::{ClipboardContext, ClipboardProvider};

/// State shared between windows
pub struct SharedState<T> {
    #[cfg(feature = "clipboard")]
    clipboard: Option<ClipboardContext>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub theme: T,
    pub pending: Vec<PendingAction>,
}

impl<T> SharedState<T> {
    /// Construct
    pub fn new(
        theme: T,
        adapter_options: Option<&wgpu::RequestAdapterOptions>,
    ) -> Result<Self, Error> {
        #[cfg(feature = "clipboard")]
        let clipboard = match ClipboardContext::new() {
            Ok(cb) => Some(cb),
            Err(e) => {
                warn!("Unable to open clipboard: {:?}", e);
                None
            }
        };

        let adapter_options = adapter_options.unwrap_or(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            backends: wgpu::BackendBit::PRIMARY,
        });

        let adapter = match wgpu::Adapter::request(adapter_options) {
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

        Ok(SharedState {
            #[cfg(feature = "clipboard")]
            clipboard,
            device,
            queue,
            theme,
            pending: vec![],
        })
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

pub enum PendingAction {
    AddWindow(Box<dyn kas::Window>),
}
