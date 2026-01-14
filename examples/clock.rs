// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Clock example
//!
//! Demonstrates low-level drawing and timer handling.
//!
//! Note that two forms of animation are possible: calling `draw.draw().animate();`
//! in `fn Clock::draw`, or using `Event::Timer`. We use the latter since
//! it lets us draw at 1 FPS with exactly the right frame time.

extern crate chrono;

use chrono::prelude::*;
use std::f32::consts::PI;
use std::time::Duration;

use kas::draw::color::Rgba;
use kas::event::TimerHandle;
use kas::geom::{Quad, Vec2};
use kas::prelude::*;
use kas::theme::{Text, TextClass};

const TIMER: TimerHandle = TimerHandle::new(0, true);

#[impl_self]
mod Clock {
    #[widget]
    struct Clock {
        core: widget_core!(),
        now: DateTime<Local>,
        date: Text<String>,
        time: Text<String>,
    }

    impl Layout for Clock {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            cx.logical(64.0, 64.0)
                .with_ideal_factor(3.0)
                .with_stretch(Stretch::High)
                .build(axis)
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, _: AlignHints) {
            // Force to square
            let size = rect.size.0.min(rect.size.1);
            let size = Size::splat(size);
            let excess = rect.size - size;
            let pos = rect.pos + excess / 2;
            self.core.set_rect(Rect { pos, size });

            let text_size = Size(size.0, size.1 / 4);
            let text_height = text_size.1 as f32;

            self.date.set_font_size(text_height * 0.5);
            self.time.set_font_size(text_height * 0.7);

            let time_pos = pos + Offset(0, size.1 * 5 / 8);
            let date_pos = pos + Offset(0, size.1 / 8);
            self.date
                .set_rect(cx, Rect::new(date_pos, text_size), AlignHints::CENTER);
            self.time
                .set_rect(cx, Rect::new(time_pos, text_size), AlignHints::CENTER);
        }

        fn draw(&self, mut cx: DrawCx) {
            let colors = cx.colors();
            let col_back = Rgba::ga(0.0, 0.5);
            let col_face = colors.accent_soft.desaturate(0.6);
            let col_time = Rgba::grey(1.0);
            let col_date = Rgba::grey(0.8);
            let col_hands = colors.accent_soft;
            let col_secs = colors.accent;

            // We use the low-level draw device to draw our clock. This means it is
            // not themeable, but gives us much more flexible draw routines.
            let draw = cx.draw_rounded().unwrap();

            let rect = self.rect();
            let quad = Quad::conv(rect);
            draw.circle(quad, 0.0, col_back);
            draw.circle(quad, 0.98, col_face);

            let half = (quad.b.1 - quad.a.1) / 2.0;
            let centre = quad.a + half;

            let w = half * 0.015625;
            let l = w * 5.0;
            let r = half - w;
            for d in 0..12 {
                let l = if d % 3 == 0 { 2.0 * l } else { l };
                let t = d as f32 * (PI / 6.0);
                let v = Vec2(t.sin(), -t.cos());
                draw.rounded_line(centre + v * (r - l), centre + v * r, w, col_face);
            }

            let mut line_seg = |t: f32, r1: f32, r2: f32, w, col| {
                let v = Vec2(t.sin(), -t.cos());
                draw.rounded_line(centre + v * r1, centre + v * r2, w, col);
            };

            let secs = self.now.time().num_seconds_from_midnight();
            let a_sec = f32::conv(secs % 60) * (PI / 30.0);
            let a_min = f32::conv(secs % 3600) * (PI / 1800.0);
            let a_hour = f32::conv(secs % 43200) * (PI / (21600.0));

            line_seg(a_hour, 0.0, half * 0.55, half * 0.03, col_hands);
            line_seg(a_min, 0.0, half * 0.8, half * 0.02, col_hands);
            line_seg(a_sec, 0.0, half * 0.9, half * 0.01, col_secs);

            cx.text_with_color(self.date.rect(), &self.date, col_date);
            cx.text_with_color(self.time.rect(), &self.time, col_time);
        }
    }

    impl Events for Clock {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.request_timer(self.id(), TIMER, Duration::ZERO);
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Timer(TIMER) => {
                    self.now = Local::now();
                    let date = self.now.format("%Y-%m-%d").to_string();
                    let time = self.now.format("%H:%M:%S").to_string();
                    self.date.set_text(date);
                    self.date.prepare();
                    self.time.set_text(time);
                    self.time.prepare();
                    let ns = 1_000_000_000 - (self.now.time().nanosecond() % 1_000_000_000);
                    log::info!("Requesting update in {}ns", ns);
                    cx.request_timer(self.id(), TIMER, Duration::from_nanos(ns as u64));
                    cx.redraw();
                    Used
                }
                _ => Unused,
            }
        }
    }

    impl Clock {
        fn new() -> Self {
            Clock {
                core: Default::default(),
                now: Local::now(),
                date: Text::new("0000-00-00".to_string(), TextClass::Label, false),
                time: Text::new("00:00:00".to_string(), TextClass::Label, false),
            }
        }
    }
}

#[cfg(feature = "wgpu")]
fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let window = Window::new(Clock::new(), "Clock")
        .with_decorations(kas::window::Decorations::None)
        .with_transparent(true);

    kas::runner::Runner::with_theme(kas::theme::FlatTheme::default())
        .build(())?
        .with(window)
        .run()
}

#[cfg(not(feature = "wgpu"))]
fn main() {
    eprintln!("This example requires feature wgpu!");
}
