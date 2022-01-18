// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use log::info;
use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Duration;

use crate::draw::{CustomPipe, CustomPipeBuilder, DrawPipe, DrawWindow};
use crate::{warn_about_error, Error, Options, WindowId};
use kas::cast::Conv;
use kas::draw;
use kas::event::UpdateHandle;
use kas::TkAction;
use kas_theme::{Theme, ThemeConfig};

#[cfg(feature = "clipboard")]
use window_clipboard::Clipboard;

/// State shared between windows
pub struct SharedState<C: CustomPipe, T> {
    #[cfg(feature = "clipboard")]
    clipboard: Option<Clipboard>,
    pub instance: wgpu::Instance,
    pub draw: draw::SharedState<DrawPipe<C>>,
    pub theme: T,
    pub config: Rc<RefCell<kas::event::Config>>,
    pub pending: Vec<PendingAction>,
    /// Newly created windows need to know the scale_factor *before* they are
    /// created. This is used to estimate ideal window size.
    pub scale_factor: f64,
    pub frame_dur: Duration,
    window_id: u32,
    options: Options,
}

impl<C: CustomPipe, T: Theme<DrawPipe<C>>> SharedState<C, T>
where
    T::Window: kas_theme::Window,
{
    /// Construct
    pub fn new<CB: CustomPipeBuilder<Pipe = C>>(
        custom: CB,
        mut theme: T,
        options: Options,
        config: Rc<RefCell<kas::event::Config>>,
        scale_factor: f64,
    ) -> Result<Self, Error> {
        let instance = wgpu::Instance::new(options.backend());
        let adapter_options = options.adapter_options();
        let req = instance.request_adapter(&adapter_options);
        let adapter = match futures::executor::block_on(req) {
            Some(a) => a,
            None => return Err(Error::NoAdapter),
        };
        info!("Using graphics adapter: {}", adapter.get_info().name);

        let desc = CB::device_descriptor();
        let trace_path = options.wgpu_trace_path.as_deref();
        let req = adapter.request_device(&desc, trace_path);
        let device_and_queue = futures::executor::block_on(req)?;

        let pipe = DrawPipe::new(custom, device_and_queue, theme.config().raster());
        let mut draw = draw::SharedState::new(pipe);

        theme.init(&mut draw);

        let mut frame_dur = Duration::new(0, 0);
        if let Some(limit) = options.fps_limit {
            frame_dur = Duration::from_secs_f64(1.0 / f64::conv(limit.get()));
        }

        Ok(SharedState {
            #[cfg(feature = "clipboard")]
            clipboard: None,
            instance,
            draw,
            theme,
            config,
            pending: vec![],
            scale_factor,
            frame_dur,
            window_id: 0,
            options,
        })
    }

    /// Initialise the clipboard context
    ///
    /// This requires a window handle (on some platforms), thus is done when the
    /// first window is constructed.
    pub fn init_clipboard(&mut self, _window: &winit::window::Window) {
        #[cfg(feature = "clipboard")]
        if self.clipboard.is_none() {
            match Clipboard::connect(_window) {
                Ok(cb) => self.clipboard = Some(cb),
                Err(e) => warn_about_error("Failed to connect clipboard", e.as_ref()),
            }
        }
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
    ) {
        self.draw.draw.render(window, frame_view, clear_color);
    }

    #[inline]
    pub fn get_clipboard(&mut self) -> Option<String> {
        #[cfg(feature = "clipboard")]
        {
            self.clipboard.as_ref().and_then(|cb| match cb.read() {
                Ok(c) => Some(c),
                Err(e) => {
                    warn_about_error("Failed to get clipboard contents", e.as_ref());
                    None
                }
            })
        }
        #[cfg(not(feature = "clipboard"))]
        None
    }

    #[inline]
    pub fn set_clipboard(&mut self, _content: String) {
        #[cfg(feature = "clipboard")]
        if let Some(cb) = self.clipboard.as_mut() {
            match cb.write(_content) {
                Ok(()) => (),
                Err(e) => warn_about_error("Failed to set clipboard contents", e.as_ref()),
            }
        }
    }

    pub fn trigger_update(&mut self, handle: UpdateHandle, payload: u64) {
        self.pending.push(PendingAction::Update(handle, payload));
    }

    pub fn on_exit(&self) {
        match self
            .options
            .write_config(&self.config.borrow(), &self.theme)
        {
            Ok(()) => (),
            Err(error) => warn_about_error("Failed to save config", &error),
        }
    }
}

pub enum PendingAction {
    AddPopup(winit::window::WindowId, WindowId, kas::Popup),
    AddWindow(WindowId, Box<dyn kas::Window>),
    CloseWindow(WindowId),
    Update(kas::event::UpdateHandle, u64),
    TkAction(TkAction),
}
