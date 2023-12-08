// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Application, platforms and backends

#[cfg(winit)] mod app;
mod common;
#[cfg(winit)] mod event_loop;
#[cfg(winit)] mod shared;
#[cfg(winit)] mod window;

#[cfg(winit)] use crate::WindowId;
use crate::{Action, ErasedStack};
#[cfg(winit)] use app::PlatformWrapper;
#[cfg(winit)] use event_loop::Loop as EventLoop;
#[cfg(winit)] pub(crate) use shared::{AppShared, AppState};
#[cfg(winit)]
pub(crate) use window::{Window, WindowDataErased};

#[cfg(winit)]
pub use app::{AppAssoc, AppBuilder, Application, ClosedError, Proxy};
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub use common::{AppGraphicsBuilder, WindowSurface};
pub use common::{Error, Platform, Result};

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub extern crate raw_window_handle;

/// Application state
///
/// Kas allows application state to be stored both in the  widget tree (in
/// `Adapt` nodes and user-defined widgets) and by the application root (shared
/// across windows). This trait must be implemented by the latter.
///
/// When no top-level data is required, use `()` which implements this trait.
pub trait AppData: 'static {
    /// Handle messages
    ///
    /// This is the last message handler: it is called when, after traversing
    /// the widget tree (see [kas::event] module doc), a message is left on the
    /// stack. Unhandled messages will result in warnings in the log.
    ///
    /// The method returns an [`Action`], usually either [`Action::empty`]
    /// (nothing to do) or [`Action::UPDATE`] (to update widgets).
    /// This action affects all windows.
    fn handle_messages(&mut self, messages: &mut ErasedStack) -> Action;
}

impl AppData for () {
    fn handle_messages(&mut self, _: &mut ErasedStack) -> Action {
        Action::empty()
    }
}

#[crate::autoimpl(Debug)]
#[cfg(winit)]
enum Pending<A: AppData, S: WindowSurface, T: kas::theme::Theme<S::Shared>> {
    AddPopup(WindowId, WindowId, kas::PopupDescriptor),
    // NOTE: we don't need S, T here if we construct the Window later.
    // But this way we can pass a single boxed value.
    AddWindow(WindowId, Box<Window<A, S, T>>),
    CloseWindow(WindowId),
    Action(kas::Action),
}

#[cfg(winit)]
#[derive(Debug)]
enum ProxyAction {
    CloseAll,
    Close(WindowId),
    Message(kas::erased::SendErased),
    WakeAsync,
}

#[cfg(test)]
mod test {
    use super::*;
    use raw_window_handle as raw;
    use std::time::Instant;

    struct Draw;
    impl crate::draw::DrawImpl for Draw {
        fn common_mut(&mut self) -> &mut crate::draw::WindowCommon {
            todo!()
        }

        fn new_pass(
            &mut self,
            _: crate::draw::PassId,
            _: crate::prelude::Rect,
            _: crate::prelude::Offset,
            _: crate::draw::PassType,
        ) -> crate::draw::PassId {
            todo!()
        }

        fn get_clip_rect(&self, _: crate::draw::PassId) -> crate::prelude::Rect {
            todo!()
        }

        fn rect(
            &mut self,
            _: crate::draw::PassId,
            _: crate::geom::Quad,
            _: crate::draw::color::Rgba,
        ) {
            todo!()
        }

        fn frame(
            &mut self,
            _: crate::draw::PassId,
            _: crate::geom::Quad,
            _: crate::geom::Quad,
            _: crate::draw::color::Rgba,
        ) {
            todo!()
        }
    }
    impl crate::draw::DrawRoundedImpl for Draw {
        fn rounded_line(
            &mut self,
            _: crate::draw::PassId,
            _: crate::geom::Vec2,
            _: crate::geom::Vec2,
            _: f32,
            _: crate::draw::color::Rgba,
        ) {
            todo!()
        }

        fn circle(
            &mut self,
            _: crate::draw::PassId,
            _: crate::geom::Quad,
            _: f32,
            _: crate::draw::color::Rgba,
        ) {
            todo!()
        }

        fn circle_2col(
            &mut self,
            _: crate::draw::PassId,
            _: crate::geom::Quad,
            _: crate::draw::color::Rgba,
            _: crate::draw::color::Rgba,
        ) {
            todo!()
        }

        fn rounded_frame(
            &mut self,
            _: crate::draw::PassId,
            _: crate::geom::Quad,
            _: crate::geom::Quad,
            _: f32,
            _: crate::draw::color::Rgba,
        ) {
            todo!()
        }

        fn rounded_frame_2col(
            &mut self,
            _: crate::draw::PassId,
            _: crate::geom::Quad,
            _: crate::geom::Quad,
            _: crate::draw::color::Rgba,
            _: crate::draw::color::Rgba,
        ) {
            todo!()
        }
    }

    struct DrawShared;
    impl crate::draw::DrawSharedImpl for DrawShared {
        type Draw = Draw;

        fn max_texture_dimension_2d(&self) -> u32 {
            todo!()
        }

        fn set_raster_config(&mut self, _: &crate::theme::RasterConfig) {
            todo!()
        }

        fn image_alloc(
            &mut self,
            _: (u32, u32),
        ) -> std::result::Result<crate::draw::ImageId, crate::draw::AllocError> {
            todo!()
        }

        fn image_upload(&mut self, _: crate::draw::ImageId, _: &[u8], _: crate::draw::ImageFormat) {
            todo!()
        }

        fn image_free(&mut self, _: crate::draw::ImageId) {
            todo!()
        }

        fn image_size(&self, _: crate::draw::ImageId) -> Option<(u32, u32)> {
            todo!()
        }

        fn draw_image(
            &self,
            _: &mut Self::Draw,
            _: crate::draw::PassId,
            _: crate::draw::ImageId,
            _: crate::geom::Quad,
        ) {
            todo!()
        }

        fn draw_text(
            &mut self,
            _: &mut Self::Draw,
            _: crate::draw::PassId,
            _: crate::prelude::Rect,
            _: &kas_text::TextDisplay,
            _: crate::draw::color::Rgba,
        ) {
            todo!()
        }

        fn draw_text_effects(
            &mut self,
            _: &mut Self::Draw,
            _: crate::draw::PassId,
            _: crate::prelude::Rect,
            _: &kas_text::TextDisplay,
            _: crate::draw::color::Rgba,
            _: &[kas_text::Effect<()>],
        ) {
            todo!()
        }

        fn draw_text_effects_rgba(
            &mut self,
            _: &mut Self::Draw,
            _: crate::draw::PassId,
            _: crate::prelude::Rect,
            _: &kas_text::TextDisplay,
            _: &[kas_text::Effect<crate::draw::color::Rgba>],
        ) {
            todo!()
        }
    }

    struct Surface;
    impl WindowSurface for Surface {
        type Shared = DrawShared;

        fn new<W>(_: &mut Self::Shared, _: W) -> Result<Self>
        where
            W: raw::HasRawWindowHandle + raw::HasRawDisplayHandle,
            Self: Sized,
        {
            todo!()
        }

        fn size(&self) -> crate::prelude::Size {
            todo!()
        }

        fn do_resize(&mut self, _: &mut Self::Shared, _: crate::prelude::Size) -> bool {
            todo!()
        }

        fn draw_iface<'iface>(
            &'iface mut self,
            _: &'iface mut crate::draw::SharedState<Self::Shared>,
        ) -> crate::draw::DrawIface<'iface, Self::Shared> {
            todo!()
        }

        fn common_mut(&mut self) -> &mut crate::draw::WindowCommon {
            todo!()
        }

        fn present(&mut self, _: &mut Self::Shared, _: crate::draw::color::Rgba) -> Instant {
            todo!()
        }
    }

    #[test]
    fn size_of_pending() {
        assert_eq!(
            std::mem::size_of::<Pending<(), Surface, crate::theme::SimpleTheme>>(),
            32
        );
    }
}
