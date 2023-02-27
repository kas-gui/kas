// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared state

use std::num::NonZeroU32;
use std::task::Waker;

use super::{PendingAction, Platform, WindowSurface};
use kas::config::Options;
use kas::event::UpdateId;
use kas::model::SharedRc;
use kas::shell::Error;
use kas::theme::Theme;
use kas::util::warn_about_error;
use kas::{draw, WindowId};

#[cfg(feature = "clipboard")]
use copypasta::{ClipboardContext, ClipboardProvider};

/// State shared between windows
pub struct SharedState<S: WindowSurface, T> {
    pub(super) platform: Platform,
    #[cfg(feature = "clipboard")]
    clipboard: Option<ClipboardContext>,
    pub(super) draw: draw::SharedState<S::Shared>,
    pub(super) theme: T,
    pub(super) config: SharedRc<kas::event::Config>,
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
        config: SharedRc<kas::event::Config>,
    ) -> Result<Self, Error> {
        let platform = pw.platform();
        let mut draw = kas::draw::SharedState::new(draw_shared, platform);
        theme.init(&mut draw);

        #[cfg(feature = "clipboard")]
        let clipboard = match ClipboardContext::new() {
            Ok(cb) => Some(cb),
            Err(e) => {
                warn_about_error("Failed to connect clipboard", e.as_ref());
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
                match cb.get_contents() {
                    Ok(s) => return Some(s),
                    Err(e) => warn_about_error("Failed to get clipboard contents", e.as_ref()),
                }
            }
        }

        None
    }

    #[inline]
    pub fn set_clipboard(&mut self, _content: String) {
        #[cfg(feature = "clipboard")]
        if let Some(cb) = self.clipboard.as_mut() {
            match cb.set_contents(_content) {
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
