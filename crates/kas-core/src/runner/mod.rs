// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Runner, platforms and backends

mod common;
#[cfg(winit)] mod event_loop;
#[cfg(winit)] mod runner;
#[cfg(winit)] mod shared;
#[cfg(winit)] mod window;

use crate::messages::MessageStack;
#[cfg(winit)] use crate::WindowId;
#[cfg(winit)] use event_loop::Loop;
#[cfg(winit)] use runner::PlatformWrapper;
#[cfg(winit)] pub(crate) use shared::RunnerT;
#[cfg(winit)] use shared::State;
#[cfg(winit)]
pub(crate) use window::{Window, WindowDataErased};

pub use common::{Error, Platform, Result};
#[cfg(winit)]
pub use runner::{Builder, ClosedError, Proxy, Runner, RunnerInherent};

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub use common::{GraphicsBuilder, WindowSurface};

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub extern crate raw_window_handle;

/// Application state
///
/// Kas allows application state to be stored both in the  widget tree (in
/// `Adapt` nodes and user-defined widgets) and by the application root (shared
/// across windows). This trait must be implemented by the latter.
///
/// When no top-level data is required, use `()` which implements this trait.
///
/// TODO: should we pass some type of interface to the runner to these methods?
/// We could pass a `&mut dyn RunnerT` easily, but that trait is not public.
pub trait AppData: 'static {
    /// Handle messages
    ///
    /// This is the last message handler: it is called when, after traversing
    /// the widget tree (see [kas::event] module doc), a message is left on the
    /// stack. Unhandled messages will result in warnings in the log.
    fn handle_messages(&mut self, messages: &mut MessageStack);

    /// Application is being suspended
    ///
    /// The application should ensure any important state is saved.
    ///
    /// This method is called when the application has been suspended or is
    /// about to exit (on Android/iOS/Web platforms, the application may resume
    /// after this method is called; on other platforms this probably indicates
    /// imminent closure). Widget state may still exist, but is not live
    /// (widgets will not process events or messages).
    fn suspended(&mut self) {}
}

impl AppData for () {
    fn handle_messages(&mut self, _: &mut MessageStack) {}
    fn suspended(&mut self) {}
}

#[crate::autoimpl(Debug)]
#[cfg(winit)]
enum Pending<A: AppData, G: GraphicsBuilder, T: kas::theme::Theme<G::Shared>> {
    AddPopup(WindowId, WindowId, kas::PopupDescriptor),
    // NOTE: we don't need G, T here if we construct the Window later.
    // But this way we can pass a single boxed value.
    AddWindow(WindowId, Box<Window<A, G, T>>),
    CloseWindow(WindowId),
    Action(kas::Action),
    Exit,
}

#[cfg(winit)]
#[derive(Debug)]
enum ProxyAction {
    CloseAll,
    Close(WindowId),
    Message(kas::messages::SendErased),
    WakeAsync,
}

#[cfg(test)]
mod test {
    use super::*;
    use raw_window_handle as rwh;
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

        fn set_raster_config(&mut self, _: &crate::config::RasterConfig) {
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

        fn size(&self) -> crate::prelude::Size {
            todo!()
        }

        fn configure(&mut self, _: &mut Self::Shared, _: crate::prelude::Size) -> bool {
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

    struct AGB;
    impl GraphicsBuilder for AGB {
        type DefaultTheme = crate::theme::SimpleTheme;

        type Shared = DrawShared;

        type Surface<'a> = Surface;

        fn build(self) -> Result<Self::Shared> {
            todo!()
        }

        fn new_surface<'window, W>(
            _: &mut Self::Shared,
            _: W,
            _: bool,
        ) -> Result<Self::Surface<'window>>
        where
            W: rwh::HasWindowHandle + rwh::HasDisplayHandle + Send + Sync + 'window,
            Self: Sized,
        {
            todo!()
        }
    }

    #[test]
    fn size_of_pending() {
        assert_eq!(
            std::mem::size_of::<Pending<(), AGB, crate::theme::SimpleTheme>>(),
            32
        );
    }
}
