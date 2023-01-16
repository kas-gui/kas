// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! WGPU window surface

use crate::draw::{CustomPipe, DrawPipe, DrawWindow};
use kas::cast::Cast;
use kas::draw::color::Rgba;
use kas::draw::{AnimationState, DrawIface};
use kas::geom::Size;
use raw_window_handle as raw;
use std::time::Instant;

/// Per-window data
pub struct Surface<C: CustomPipe> {
    surface: wgpu::Surface,
    sc_desc: wgpu::SurfaceConfiguration,
    draw: DrawWindow<C::Window>,
}

impl<C: CustomPipe> super::WindowSurface for Surface<C> {
    type Shared = DrawPipe<C>;

    fn new<W: raw::HasRawWindowHandle + raw::HasRawDisplayHandle>(
        shared: &mut Self::Shared,
        size: Size,
        window: W,
    ) -> Self {
        let mut draw = shared.new_window();
        shared.resize(&mut draw, size);

        let surface = unsafe { shared.instance.create_surface(&window) };
        let sc_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: crate::draw::RENDER_TEX_FORMAT,
            width: size.0.cast(),
            height: size.1.cast(),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };
        surface.configure(&shared.device, &sc_desc);

        Surface {
            surface,
            sc_desc,
            draw,
        }
    }

    fn size(&self) -> Size {
        Size::new(self.sc_desc.width.cast(), self.sc_desc.height.cast())
    }

    fn do_resize(&mut self, shared: &mut Self::Shared, size: Size) -> bool {
        if size == self.size() {
            return false;
        }
        let time = Instant::now();

        shared.resize(&mut self.draw, size);

        self.sc_desc.width = size.0.cast();
        self.sc_desc.height = size.1.cast();
        self.surface.configure(&shared.device, &self.sc_desc);

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

    fn take_animation_state(&mut self) -> AnimationState {
        std::mem::take(&mut self.draw.animation)
    }

    fn present(&mut self, shared: &mut Self::Shared, clear_color: Rgba) -> Result<u128, ()> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                log::error!("do_draw: failed to get frame texture: {}", e);
                // It may be possible to recover by calling surface.configure(...) then retrying
                // surface.get_current_texture(), but is doing so ever useful?
                return Err(());
            }
        };
        // TODO: check frame.suboptimal ?
        let view = frame.texture.create_view(&Default::default());

        let clear_color = to_wgpu_color(clear_color);
        shared.render(&mut self.draw, &view, clear_color);

        frame.present();

        Ok(self.draw.text.dur_micros())
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
