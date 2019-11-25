// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget rendering

use std::f32;

use wgpu_glyph::GlyphBrush;

use kas::geom::{AxisInfo, Margins, Size, SizeRules};
use kas::{event, TkAction, TkWindow, Widget};

use crate::round_pipe::Rounded;
use crate::theme::Theme;
use crate::tri_pipe::TriPipe;

/// Widget renderer
pub(crate) struct Widgets<T: Theme> {
    round_pipe: Rounded,
    tri_pipe: TriPipe,
    pub(crate) glyph_brush: GlyphBrush<'static, ()>,
    action: TkAction,
    pub(crate) ev_mgr: event::Manager,
    theme: T,
}

impl<T: Theme> Widgets<T> {
    pub fn new(
        device: &wgpu::Device,
        size: Size,
        dpi_factor: f64,
        glyph_brush: GlyphBrush<'static, ()>,
        mut theme: T,
    ) -> Self {
        let round_pipe = Rounded::new(device, size);
        let tri_pipe = TriPipe::new(device, size);
        theme.set_dpi_factor(dpi_factor as f32);

        Widgets {
            round_pipe,
            tri_pipe,
            glyph_brush,
            action: TkAction::None,
            ev_mgr: event::Manager::new(dpi_factor),
            theme,
        }
    }

    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        self.ev_mgr.set_dpi_factor(dpi_factor);
        self.theme.set_dpi_factor(dpi_factor as f32);
        // Note: we rely on caller to resize widget
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Size) -> wgpu::CommandBuffer {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        self.tri_pipe.resize(device, &mut encoder, size);
        self.round_pipe.resize(device, &mut encoder, size);
        encoder.finish()
    }

    #[inline]
    pub fn pop_action(&mut self) -> TkAction {
        let action = self.action;
        self.action = TkAction::None;
        action
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        rpass: &mut wgpu::RenderPass,
        win: &dyn kas::Window,
    ) {
        self.draw_iter(win.as_widget());
        self.tri_pipe.render(device, rpass);
        self.round_pipe.render(device, rpass);
    }

    fn draw_iter(&mut self, widget: &dyn kas::Widget) {
        // draw widget; recurse over children
        self.draw_widget(widget);

        for n in 0..widget.len() {
            self.draw_iter(widget.get(n).unwrap());
        }
    }

    fn draw_widget(&mut self, widget: &dyn kas::Widget) {
        self.theme.draw(
            &mut self.tri_pipe,
            &mut self.round_pipe,
            &mut self.glyph_brush,
            &self.ev_mgr,
            widget,
        )
    }
}

impl<T: Theme> TkWindow for Widgets<T> {
    fn data(&self) -> &event::Manager {
        &self.ev_mgr
    }

    fn update_data(&mut self, f: &mut dyn FnMut(&mut event::Manager) -> bool) {
        if f(&mut self.ev_mgr) {
            self.send_action(TkAction::Redraw);
        }
    }

    fn size_rules(&mut self, widget: &dyn Widget, axis: AxisInfo) -> SizeRules {
        self.theme.size_rules(&mut self.glyph_brush, widget, axis)
    }

    fn margins(&self, widget: &dyn Widget) -> Margins {
        self.theme.margins(widget)
    }

    #[inline]
    fn redraw(&mut self, _: &dyn Widget) {
        self.send_action(TkAction::Redraw);
    }

    #[inline]
    fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }
}
