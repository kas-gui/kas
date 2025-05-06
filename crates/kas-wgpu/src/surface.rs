// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! WGPU window surface

use crate::draw::{CustomPipe, DrawPipe, DrawWindow};
use kas::cast::Cast;
use kas::draw::color::Rgba;
use kas::draw::{DrawIface, DrawSharedImpl, WindowCommon};
use kas::geom::Size;
use kas::runner::{raw_window_handle as rwh, Error, WindowSurface};
use std::time::Instant;

/// Per-window data
pub struct Surface<'a, C: CustomPipe> {
    surface: wgpu::Surface<'a>,
    size: Size,
    transparent: bool,
    draw: DrawWindow<C::Window>,
}

impl<'a, C: CustomPipe> Surface<'a, C> {
    pub fn new<W>(
        shared: &mut <Self as WindowSurface>::Shared,
        window: W,
        transparent: bool,
    ) -> Result<Self, Error>
    where
        W: rwh::HasWindowHandle + rwh::HasDisplayHandle + Send + Sync + 'a,
        Self: Sized,
    {
        let surface = shared
            .instance
            .create_surface(window)
            .map_err(|e| Error::Graphics(Box::new(e)))?;

        Ok(Surface {
            surface,
            size: Size::ZERO,
            transparent,
            draw: shared.new_window(),
        })
    }
}

impl<'a, C: CustomPipe> WindowSurface for Surface<'a, C> {
    type Shared = DrawPipe<C>;

    fn size(&self) -> Size {
        self.size
    }

    fn configure(&mut self, shared: &mut Self::Shared, size: Size) -> bool {
        if size == self.size() {
            return false;
        }
        let size = size.min(Size::splat(shared.max_texture_dimension_2d().cast()));
        self.size = size;

        let time = Instant::now();

        shared.resize(&mut self.draw, size);

        use wgpu::CompositeAlphaMode::{Inherit, Opaque, PostMultiplied, PreMultiplied};
        let caps = self.surface.get_capabilities(&shared.adapter);
        let alpha_mode = match self.transparent {
            // FIXME: data conversion is needed somewhere:
            true if caps.alpha_modes.contains(&PreMultiplied) => PreMultiplied,
            true if caps.alpha_modes.contains(&PostMultiplied) => PostMultiplied,
            _ if caps.alpha_modes.contains(&Opaque) => Opaque,
            _ => Inherit, // it is specified that either Opaque or Inherit is supported
        };
        log::debug!("Surface::new: using alpha_mode={alpha_mode:?}");

        let sc_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: crate::draw::RENDER_TEX_FORMAT,
            width: size.0.cast(),
            height: size.1.cast(),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };

        self.surface.configure(&shared.device, &sc_desc);

        log::trace!(
            target: "kas_perf::wgpu::window",
            "do_resize: {}µs",
            time.elapsed().as_micros()
        );
        true
    }

    fn draw_iface<'iface>(
        &'iface mut self,
        shared: &'iface mut kas::draw::SharedState<Self::Shared>,
    ) -> DrawIface<'iface, Self::Shared> {
        DrawIface::new(&mut self.draw, shared)
    }

    fn common_mut(&mut self) -> &mut WindowCommon {
        &mut self.draw.common
    }

    /// Return time at which render finishes
    fn present(&mut self, shared: &mut Self::Shared, clear_color: Rgba) -> Instant {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                // This error has not been observed. Can it be fixed by
                // re-configuring the surface? Does it ever occur anyway?
                log::error!("WindowSurface::present: failed to get frame texture: {e}");
                return Instant::now();
            }
        };

        #[cfg(debug_assertions)]
        if frame.suboptimal {
            // Does this ever occur? Should we care?
            log::warn!("WindowSurface::present: sub-optimal frame should be re-created");
        }

        let view = frame.texture.create_view(&Default::default());

        let clear_color = to_wgpu_color(clear_color);
        shared.render(&mut self.draw, &view, clear_color);

        let pre_present = Instant::now();
        frame.present();
        pre_present
    }
}

fn to_wgpu_color(c: Rgba) -> wgpu::Color {
    wgpu::Color {
        r: c.r as f64,
        g: c.g as f64,
        b: c.b as f64,
        a: c.a as f64,
    }
}
