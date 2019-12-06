// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use std::time::{Duration, Instant};

#[cfg(feature = "clipboard")]
use clipboard::{ClipboardContext, ClipboardProvider};

use kas::event::Callback;
use kas::geom::Size;
use kas::{event, layout, theme, TkAction, Widget, WidgetId};
use winit::dpi::LogicalSize;
use winit::error::OsError;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::draw::DrawPipe;
use crate::SharedState;

/// Per-window data
pub(crate) struct Window<TW> {
    widget: Box<dyn kas::Window>,
    /// The winit window
    pub(crate) window: winit::window::Window,
    surface: wgpu::Surface,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    timeouts: Vec<(usize, Instant, Option<Duration>)>,
    tk_window: TkWindow<TW>,
}

// Public functions, for use by the toolkit
impl<TW: theme::Window<DrawPipe> + 'static> Window<TW> {
    /// Construct a window
    pub fn new<T: theme::Theme<DrawPipe, Window = TW>, U: 'static>(
        shared: &mut SharedState<T>,
        event_loop: &EventLoopWindowTarget<U>,
        mut widget: Box<dyn kas::Window>,
    ) -> Result<Self, OsError> {
        let window = winit::window::Window::new(event_loop)?;
        let dpi_factor = window.hidpi_factor();
        let size: Size = window.inner_size().to_physical(dpi_factor).into();

        let surface = wgpu::Surface::create(&window);

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.0,
            height: size.1,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = shared.device.create_swap_chain(&surface, &sc_desc);

        let mut tk_window = TkWindow::new(shared, sc_desc.format, size, dpi_factor);
        tk_window.ev_mgr.configure(widget.as_widget_mut());

        widget.resize(&mut tk_window, size);

        let w = Window {
            widget,
            window,
            surface,
            sc_desc,
            swap_chain,
            timeouts: vec![],
            tk_window,
        };

        Ok(w)
    }

    /// Called by the `Toolkit` when the event loop starts to initialise
    /// windows. Optionally returns a callback time.
    pub fn init(&mut self) -> Option<Instant> {
        self.window.request_redraw();

        for (i, condition) in self.widget.callbacks() {
            match condition {
                Callback::Start => {
                    self.widget.trigger_callback(i, &mut self.tk_window);
                }
                Callback::Repeat(dur) => {
                    self.widget.trigger_callback(i, &mut self.tk_window);
                    self.timeouts.push((i, Instant::now() + dur, Some(dur)));
                }
            }
        }

        self.next_resume()
    }

    /// Recompute layout of widgets and redraw
    pub fn reconfigure(&mut self) {
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        self.widget.resize(&mut self.tk_window, size);
        self.window.request_redraw();
    }

    /// Handle an event
    ///
    /// Return true to remove the window
    pub fn handle_event<T: theme::Theme<DrawPipe, Window = TW>>(
        &mut self,
        shared: &mut SharedState<T>,
        event: WindowEvent,
    ) -> TkAction {
        // Note: resize must be handled here to update self.swap_chain.
        match event {
            WindowEvent::Resized(size) => self.do_resize(shared, size),
            WindowEvent::RedrawRequested => self.do_draw(shared),
            WindowEvent::HiDpiFactorChanged(factor) => {
                self.tk_window.set_dpi_factor(factor);
                self.do_resize(shared, self.window.inner_size());
            }
            event @ _ => {
                event::Manager::handle_winit(&mut *self.widget, &mut self.tk_window, event)
            }
        }
        self.tk_window.pop_action()
    }

    pub(crate) fn timer_resume(&mut self, instant: Instant) -> (TkAction, Option<Instant>) {
        // Iterate over loop, mutating some elements, removing others.
        let mut i = 0;
        while i < self.timeouts.len() {
            for timeout in &mut self.timeouts[i..] {
                if timeout.1 == instant {
                    self.widget.trigger_callback(timeout.0, &mut self.tk_window);
                    if let Some(dur) = timeout.2 {
                        while timeout.1 <= Instant::now() {
                            timeout.1 += dur;
                        }
                    } else {
                        break; // remove
                    }
                }
                i += 1;
            }
            if i < self.timeouts.len() {
                self.timeouts.remove(i);
            }
        }

        (self.tk_window.pop_action(), self.next_resume())
    }

    fn next_resume(&self) -> Option<Instant> {
        let mut next = None;
        for timeout in &self.timeouts {
            next = match next {
                None => Some(timeout.1),
                Some(t) => Some(t.min(timeout.1)),
            }
        }
        next
    }
}

// Internal functions
impl<TW: theme::Window<DrawPipe> + 'static> Window<TW> {
    fn do_resize<T: theme::Theme<DrawPipe, Window = TW>>(
        &mut self,
        shared: &mut SharedState<T>,
        size: LogicalSize,
    ) {
        let size = size.to_physical(self.window.hidpi_factor()).into();
        if size == Size(self.sc_desc.width, self.sc_desc.height) {
            return;
        }
        self.widget.resize(&mut self.tk_window, size);

        let buf = self.tk_window.resize(&shared.device, size);
        shared.queue.submit(&[buf]);

        self.sc_desc.width = size.0;
        self.sc_desc.height = size.1;
        self.swap_chain = shared
            .device
            .create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn do_draw<T: theme::Theme<DrawPipe, Window = TW>>(&mut self, shared: &mut SharedState<T>) {
        let frame = self.swap_chain.get_next_texture();
        let mut draw_handle = unsafe {
            shared.theme.draw_handle(
                &mut self.tk_window.draw_pipe,
                &mut self.tk_window.theme_window,
            )
        };
        self.tk_window
            .draw_iter(&mut draw_handle, self.widget.as_widget());
        let buf = self.tk_window.render(shared, &frame.view);
        shared.queue.submit(&[buf]);
    }
}

/// Implementation of [`kas::TkWindow`]
pub(crate) struct TkWindow<TW> {
    #[cfg(feature = "clipboard")]
    clipboard: Option<ClipboardContext>,
    draw_pipe: DrawPipe,
    action: TkAction,
    pub(crate) ev_mgr: event::Manager,
    theme_window: TW,
}

impl<TW: theme::Window<DrawPipe> + 'static> TkWindow<TW> {
    pub fn new<T: theme::Theme<DrawPipe, Window = TW>>(
        shared: &mut SharedState<T>,
        tex_format: wgpu::TextureFormat,
        size: Size,
        dpi_factor: f64,
    ) -> Self {
        #[cfg(feature = "clipboard")]
        let clipboard = match ClipboardContext::new() {
            Ok(cb) => Some(cb),
            Err(e) => {
                // TODO: use log
                println!("Warning: unable to open clipboard: {:?}", e);
                None
            }
        };

        let mut draw_pipe = DrawPipe::new(&mut shared.device, tex_format, size, &shared.theme);
        let theme_window = shared.theme.new_window(&mut draw_pipe, dpi_factor as f32);

        TkWindow {
            #[cfg(feature = "clipboard")]
            clipboard,
            draw_pipe,
            action: TkAction::None,
            ev_mgr: event::Manager::new(dpi_factor),
            theme_window,
        }
    }

    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        self.ev_mgr.set_dpi_factor(dpi_factor);
        self.theme_window.set_dpi_factor(dpi_factor as f32);
        // Note: we rely on caller to resize widget
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Size) -> wgpu::CommandBuffer {
        self.draw_pipe.resize(device, size)
    }

    #[inline]
    pub fn pop_action(&mut self) -> TkAction {
        let action = self.action;
        self.action = TkAction::None;
        action
    }

    /// Iterate over a widget tree, queuing drawables
    pub fn draw_iter(&mut self, theme: &mut dyn theme::DrawHandle, widget: &dyn kas::Widget) {
        theme.draw(&self.ev_mgr, widget);

        for n in 0..widget.len() {
            self.draw_iter(theme, widget.get(n).unwrap());
        }
    }

    /// Render all queued drawables
    pub fn render<T: theme::Theme<DrawPipe, Window = TW>>(
        &mut self,
        shared: &mut SharedState<T>,
        frame_view: &wgpu::TextureView,
    ) -> wgpu::CommandBuffer {
        let clear_color = to_wgpu_color(shared.theme.clear_colour());
        self.draw_pipe
            .render(&mut shared.device, frame_view, clear_color)
    }
}

impl<TW: theme::Window<DrawPipe>> kas::TkWindow for TkWindow<TW> {
    fn data(&self) -> &event::Manager {
        &self.ev_mgr
    }

    fn update_data(&mut self, f: &mut dyn FnMut(&mut event::Manager) -> bool) {
        if f(&mut self.ev_mgr) {
            self.send_action(TkAction::Redraw);
        }
    }

    fn margins(&self, widget: &dyn Widget) -> layout::Margins {
        self.theme_window.margins(widget)
    }

    fn size_rules(&mut self, widget: &dyn Widget, axis: layout::AxisInfo) -> layout::SizeRules {
        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw_pipe) };
        use kas::theme::SizeHandle;
        size_handle.size_rules(widget, axis)
    }

    #[inline]
    fn redraw(&mut self, _id: WidgetId) {
        self.send_action(TkAction::Redraw);
    }

    #[inline]
    fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }

    #[cfg(not(feature = "clipboard"))]
    #[inline]
    fn get_clipboard(&mut self) -> Option<String> {
        None
    }

    #[cfg(feature = "clipboard")]
    fn get_clipboard(&mut self) -> Option<String> {
        self.clipboard
            .as_mut()
            .and_then(|cb| match cb.get_contents() {
                Ok(c) => Some(c),
                Err(e) => {
                    // TODO: use log
                    println!("Warning: failed to get clipboard contents: {:?}", e);
                    None
                }
            })
    }

    #[cfg(not(feature = "clipboard"))]
    #[inline]
    fn set_clipboard(&mut self, _content: String) {}

    #[cfg(feature = "clipboard")]
    fn set_clipboard(&mut self, content: String) {
        self.clipboard.as_mut().map(|cb| {
            cb.set_contents(content).unwrap_or_else(|e|
                // TODO: use log
                println!("Warning: failed to set clipboard contents: {:?}", e))
        });
    }
}

fn to_wgpu_color(c: kas::draw::Colour) -> wgpu::Color {
    wgpu::Color {
        r: c.r as f64,
        g: c.g as f64,
        b: c.b as f64,
        a: c.a as f64,
    }
}
