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

use std::time::Instant;

pub use draw::Shared;
use kas::draw::{DrawImpl, DrawSharedImpl, SharedState, WindowCommon, color};
use kas::geom::Size;
use kas::runner::{GraphicsInstance, RunError, WindowSurface};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// Graphics context
pub struct Instance {}

impl Instance {
    /// Construct a new `Instance`
    pub fn new() -> Self {
        Instance {}
    }
}

pub struct Surface {}

impl WindowSurface for Surface {
    type Shared = Shared;

    fn size(&self) -> Size {
        todo!()
    }

    fn configure(&mut self, shared: &mut Shared, size: Size) -> bool {
        todo!()
    }

    fn draw_iface<'iface>(
        &'iface mut self,
        shared: &'iface mut SharedState<Shared>,
    ) -> kas::draw::DrawIface<'iface, Shared> {
        todo!()
    }

    fn common_mut(&mut self) -> &mut WindowCommon {
        todo!()
    }

    fn present(&mut self, shared: &mut Shared, clear_color: color::Rgba) -> Instant {
        todo!()
    }
}

impl GraphicsInstance for Instance {
    type Shared = Shared;

    type Surface<'a> = Surface;

    fn new_shared(&mut self, surface: Option<&Surface>) -> Result<Shared, RunError> {
        Ok(Shared {})
    }

    fn new_surface<'window, W>(&mut self, window: W, transparent: bool) -> Result<Surface, RunError>
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'window,
        Self: Sized,
    {
        Ok(Surface {})
    }
}
