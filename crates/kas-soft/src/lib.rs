// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS graphics backend over [softbuffer]
//!
//! This crate implements a KAS's drawing APIs over [softbuffer].
//!
//! This crate supports themes via the [`kas::theme`].
//!
//! [softbuffer]: https://github.com/rust-windowing/softbuffer

mod atlas;
mod basic;
mod draw;

use std::num::NonZeroU32;
use std::time::Instant;

pub use draw::{Draw, Shared};
use kas::cast::Cast;
use kas::draw::{SharedState, WindowCommon, color};
use kas::geom::Size;
use kas::runner::{
    Error, GraphicsFeatures, GraphicsInstance, PresentResult, RunError, WindowSurface,
};
use kas::winit::event_loop::OwnedDisplayHandle;
use kas::winit::window::Window;
use softbuffer::Context;

/// Graphics context
pub struct Instance {
    context: Context<OwnedDisplayHandle>,
}

impl Instance {
    /// Construct a new `Instance`
    #[allow(clippy::new_without_default)]
    pub fn new(display: OwnedDisplayHandle) -> Result<Self, Error> {
        Ok(Instance {
            context: Context::new(display).map_err(|err| Error::Graphics(Box::new(err)))?,
        })
    }
}

pub struct Surface {
    size: Size,
    surface: softbuffer::Surface<OwnedDisplayHandle, Box<dyn Window>>,
    draw: Draw,
}

/// Convert to softbuffer's color representation
///
/// sRGB is assumed.
fn color_to_u32(c: impl Into<color::Rgba8Srgb>) -> u32 {
    u32::from_be_bytes(c.into().0) >> 8
}

impl WindowSurface for Surface {
    type Shared = Shared;

    fn size(&self) -> Size {
        self.size
    }

    fn configure(&mut self, _: &mut Shared, size: Size) -> bool {
        if size == self.size() {
            return false;
        }

        self.size = size;
        self.draw.resize(size);

        let (w, h) = NonZeroU32::new(size.0.cast())
            .zip(NonZeroU32::new(size.1.cast()))
            .expect("zero-sized surface");
        self.surface.resize(w, h).expect("surface resize failed");
        true
    }

    fn draw_iface<'iface>(
        &'iface mut self,
        shared: &'iface mut SharedState<Shared>,
    ) -> kas::draw::DrawIface<'iface, Shared> {
        kas::draw::DrawIface::new(&mut self.draw, shared)
    }

    fn common_mut(&mut self) -> &mut WindowCommon {
        &mut self.draw.common
    }

    fn present(&mut self, shared: &mut Shared, clear_color: color::Rgba) -> PresentResult {
        let mut buffer = match self.surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                log::error!("present surface: {e}");
                return PresentResult::Fatal;
            }
        };
        let width: usize = self.size.0.cast();
        let height: usize = self.size.1.cast();
        debug_assert_eq!(width * height, buffer.len());

        let c = color_to_u32(clear_color);
        buffer.fill(c);

        self.draw.render(shared, &mut buffer, (width, height));

        let pre_present = Instant::now();
        match buffer.present() {
            Ok(()) => PresentResult::Success(pre_present),
            Err(e) => {
                log::warn!("failed to present buffer: {e}");
                PresentResult::Dropped
            }
        }
    }

    #[inline]
    fn winit_window(&self) -> &dyn Window {
        &**self.surface.window()
    }
}

impl GraphicsInstance for Instance {
    type Shared = Shared;

    type Surface = Surface;

    fn new_shared(&mut self, _: Option<&Surface>, _: GraphicsFeatures) -> Result<Shared, RunError> {
        Ok(Shared::default())
    }

    fn new_surface(&mut self, window: Box<dyn Window>, _: bool) -> Result<Self::Surface, RunError>
    where
        Self: Sized,
    {
        let surface = softbuffer::Surface::new(&self.context, window)
            .map_err(|err| RunError::Graphics(Box::new(err)))?;

        Ok(Surface {
            size: Size::ZERO,
            surface,
            draw: Draw::default(),
        })
    }
}
