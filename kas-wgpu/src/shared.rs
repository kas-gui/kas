// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use log::{info, warn};
use std::num::NonZeroU32;

use crate::draw::{CustomPipe, CustomPipeBuilder, DrawPipe, DrawWindow, ShaderManager};
use crate::{Error, Options, WindowId};
use kas::event::UpdateHandle;
use kas_theme::Theme;

#[cfg(feature = "clipboard")]
use clipboard::{ClipboardContext, ClipboardProvider};

/// State shared between windows
pub struct SharedState<C: CustomPipe, T> {
    #[cfg(feature = "clipboard")]
    clipboard: Option<ClipboardContext>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub shaders: ShaderManager,
    pub draw: DrawPipe<C>,
    pub theme: T,
    pub pending: Vec<PendingAction>,
    window_id: u32,
}

impl<C: CustomPipe, T: Theme<DrawPipe<C>>> SharedState<C, T>
where
    T::Window: kas_theme::Window<DrawWindow<C::Window>>,
{
    /// Construct
    pub fn new<CB: CustomPipeBuilder<Pipe = C>>(
        custom: CB,
        mut theme: T,
        options: Options,
    ) -> Result<Self, Error> {
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
        let mut draw = DrawPipe::new(custom, &device, &shaders);

        theme.init(&mut draw);

        Ok(SharedState {
            #[cfg(feature = "clipboard")]
            clipboard,
            device,
            queue,
            shaders,
            draw,
            theme,
            pending: vec![],
            window_id: 0,
        })
    }

    pub fn next_window_id(&mut self) -> WindowId {
        self.window_id += 1;
        WindowId::new(NonZeroU32::new(self.window_id).unwrap())
    }

    pub fn render(
        &mut self,
        window: &mut DrawWindow<C::Window>,
        frame_view: &wgpu::TextureView,
        clear_color: wgpu::Color,
    ) -> wgpu::CommandBuffer {
        self.draw
            .render(window, &mut self.device, frame_view, clear_color)
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
    AddWindow(WindowId, Box<dyn kas::Window>),
    CloseWindow(WindowId),
    ThemeResize,
    RedrawAll,
    Update(UpdateHandle, u64),
}
