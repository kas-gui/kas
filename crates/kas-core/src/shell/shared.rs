// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use std::num::NonZeroU32;
use std::task::Waker;

use super::{PendingAction, Platform, WindowSurface};
use kas::config::Options;
use kas::shell::Error;
use kas::theme::Theme;
use kas::util::warn_about_error;
use kas::{draw, WindowId};
use std::rc::Rc;

#[cfg(feature = "clipboard")] use arboard::Clipboard;

/// State shared between windows
pub struct SharedState<S: WindowSurface, T> {
    pub(super) platform: Platform,
    #[cfg(feature = "clipboard")]
    clipboard: Option<Clipboard>,
    pub(super) draw: draw::SharedState<S::Shared>,
    pub(super) theme: T,
    pub(super) config: Rc<kas::event::Config>,
    pub(super) pending: Vec<PendingAction>,
    /// Estimated scale factor (from last window constructed or available screens)
    pub(super) scale_factor: f64,
    pub(super) waker: Waker,
    window_id: u32,
    options: Options,
}

impl<S: WindowSurface, T: Theme<S::Shared>> SharedState<S, T>
where
    T::Window: kas::theme::Window,
{
    /// Construct
    pub(super) fn new(
        pw: super::PlatformWrapper,
        draw_shared: S::Shared,
        mut theme: T,
        options: Options,
        config: Rc<kas::event::Config>,
    ) -> Result<Self, Error> {
        let platform = pw.platform();
        let mut draw = kas::draw::SharedState::new(draw_shared, platform);
        theme.init(&mut draw);

        #[cfg(feature = "clipboard")]
        let clipboard = match Clipboard::new() {
            Ok(cb) => Some(cb),
            Err(e) => {
                warn_about_error("Failed to connect clipboard", &e);
                None
            }
        };

        Ok(SharedState {
            platform,
            #[cfg(feature = "clipboard")]
            clipboard,
            draw,
            theme,
            config,
            pending: vec![],
            scale_factor: pw.guess_scale_factor(),
            waker: pw.create_waker(),
            window_id: 0,
            options,
        })
    }

    pub fn next_window_id(&mut self) -> WindowId {
        self.window_id += 1;
        WindowId::new(NonZeroU32::new(self.window_id).unwrap())
    }

    #[inline]
    pub fn get_clipboard(&mut self) -> Option<String> {
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

    #[inline]
    pub fn set_clipboard(&mut self, _content: String) {
        #[cfg(feature = "clipboard")]
        if let Some(cb) = self.clipboard.as_mut() {
            match cb.set_text(_content) {
                Ok(()) => (),
                Err(e) => warn_about_error("Failed to set clipboard contents", &e),
            }
        }
    }

    #[inline]
    pub fn get_primary(&mut self) -> Option<String> {
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

    #[inline]
    pub fn set_primary(&mut self, _content: String) {
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

    pub fn on_exit(&self) {
        match self.options.write_config(&self.config, &self.theme) {
            Ok(()) => (),
            Err(error) => warn_about_error("Failed to save config", &error),
        }
    }
}
