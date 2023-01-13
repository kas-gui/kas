// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use std::num::NonZeroU32;
use std::time::Duration;

use crate::{warn_about_error, Error, Options, WindowId, WindowSurface};
use kas::cast::Conv;
use kas::draw;
use kas::event::UpdateId;
use kas::model::SharedRc;
use kas::theme::Theme;
use kas::TkAction;

#[cfg(feature = "clipboard")]
use window_clipboard::Clipboard;

/// State shared between windows
pub struct SharedState<S: WindowSurface, T> {
    #[cfg(feature = "clipboard")]
    clipboard: Option<Clipboard>,
    pub draw: draw::SharedState<S::Shared>,
    pub theme: T,
    pub config: SharedRc<kas::event::Config>,
    pub pending: Vec<PendingAction>,
    /// Estimated scale factor (from last window constructed or available screens)
    pub scale_factor: f64,
    pub frame_dur: Duration,
    window_id: u32,
    options: Options,
}

impl<S: WindowSurface, T: Theme<S::Shared>> SharedState<S, T>
where
    T::Window: kas::theme::Window,
{
    /// Construct
    pub fn new(
        draw_shared: S::Shared,
        mut theme: T,
        options: Options,
        config: SharedRc<kas::event::Config>,
        scale_factor: f64,
    ) -> Result<Self, Error> {
        let mut draw = kas::draw::SharedState::new(draw_shared);
        theme.init(&mut draw);

        let mut frame_dur = Duration::new(0, 0);
        if let Some(limit) = options.fps_limit {
            frame_dur = Duration::from_secs_f64(1.0 / f64::conv(limit.get()));
        }

        Ok(SharedState {
            #[cfg(feature = "clipboard")]
            clipboard: None,
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

    pub fn update_all(&mut self, id: UpdateId, payload: u64) {
        self.pending.push(PendingAction::Update(id, payload));
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
    Update(kas::event::UpdateId, u64),
    TkAction(TkAction),
}
