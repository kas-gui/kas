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

mod draw;

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

pub use draw::{Draw, Shared};
use kas::cast::Cast;
use kas::draw::{DrawImpl, DrawSharedImpl, SharedState, WindowCommon, color};
use kas::geom::Size;
use kas::runner::{GraphicsInstance, HasDisplayAndWindowHandle, RunError, WindowSurface};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// Graphics context
pub struct Instance {}

impl Instance {
    /// Construct a new `Instance`
    pub fn new() -> Self {
        Instance {}
    }
}

pub struct Surface {
    size: Size,
    surface:
        softbuffer::Surface<Arc<dyn HasDisplayAndWindowHandle>, Arc<dyn HasDisplayAndWindowHandle>>,
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

    fn configure(&mut self, shared: &mut Shared, size: Size) -> bool {
        if size == self.size() {
            return false;
        }

        self.size = size;
        let width = NonZeroU32::new(size.0.cast()).expect("zero-sized surface");
        let height = NonZeroU32::new(size.1.cast()).expect("zero-sized surface");
        self.surface
            .resize(width, height)
            .expect("surface resize failed");
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

    fn present(&mut self, shared: &mut Shared, clear_color: color::Rgba) -> Instant {
        let mut buffer = self
            .surface
            .buffer_mut()
            .expect("failed to access surface buffer");
        let width: usize = self.size.0.cast();
        let height: usize = self.size.1.cast();
        debug_assert_eq!(width * height, buffer.len());

        let c = color_to_u32(clear_color);
        buffer.fill(c);

        self.draw.render(&mut buffer, (width, height));

        let pre_present = Instant::now();
        buffer.present().expect("failed to present buffer");
        pre_present
    }
}

impl GraphicsInstance for Instance {
    type Shared = Shared;

    type Surface<'a> = Surface;

    fn new_shared(&mut self, surface: Option<&Surface>) -> Result<Shared, RunError> {
        Ok(Shared::default())
    }

    fn new_surface<'window>(
        &mut self,
        window: Arc<dyn HasDisplayAndWindowHandle + Send + Sync>,
        transparent: bool,
    ) -> std::result::Result<Self::Surface<'window>, RunError>
    where
        Self: Sized,
    {
        let h = window as Arc<dyn HasDisplayAndWindowHandle>;

        let context =
            softbuffer::Context::new(h.clone()).map_err(|err| RunError::Graphics(Box::new(err)))?;
        let surface = softbuffer::Surface::new(&context, h)
            .map_err(|err| RunError::Graphics(Box::new(err)))?;

        Ok(Surface {
            size: Size::ZERO,
            surface,
            draw: Draw::default(),
        })
    }
}
