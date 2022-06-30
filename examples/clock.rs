// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Clock example
//!
//! Note that two forms of animation are possible: calling `draw.draw_device().animate();`
//! in `fn Clock::draw`, or using `Event::TimerUpdate`. We use the latter since
//! it lets us draw at 1 FPS with exactly the right frame time.

extern crate chrono;

use chrono::prelude::*;
use log::info;
use std::f32::consts::PI;
use std::time::Duration;

use kas::draw::{color, Draw, DrawIface, DrawRounded, PassType};
use kas::geom::{Offset, Quad, Vec2};
use kas::prelude::*;
use kas::shell::draw::DrawPipe;
use kas::text::util::set_text_and_prepare;

impl_scope! {
    #[derive(Clone, Debug)]
    #[widget]
    struct Clock {
        core: widget_core!(),
        date_pos: Coord,
        time_pos: Coord,
        now: DateTime<Local>,
        date: Text<String>,
        time: Text<String>,
    }

    impl Layout for Clock {
        fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            LogicalSize(100.0, 100.0)
                .to_rules_with_factor(axis, mgr.scale_factor(), 3.0)
                .with_stretch(Stretch::High)
        }

        #[inline]
        fn set_rect(&mut self, _: &mut ConfigMgr, rect: Rect, _align: AlignHints) {
            // Force to square
            let size = rect.size.0.min(rect.size.1);
            let size = Size::splat(size);
            let excess = rect.size - size;
            let pos = rect.pos + excess / 2;
            self.core.rect = Rect { pos, size };

            // Note: font size is calculated as dpem = dpp * pt_size. Instead of
            // the usual semantics we set dpp=1 (in constructor) and pt_size=dpem.
            let half_size = Size(size.0, size.1 / 2);
            self.date.update_env(|env| {
                env.set_pt_size(size.1 as f32 * 0.12);
                env.set_bounds(half_size.cast());
            });
            self.time.update_env(|env| {
                env.set_pt_size(size.1 as f32 * 0.15);
                env.set_bounds(half_size.cast());
            });
            self.date_pos = pos + Size(0, size.1 - half_size.1);
            self.time_pos = pos;
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let col_face = color::Rgba::grey(0.4);
            let col_time = color::Rgba::grey(0.0);
            let col_date = color::Rgba::grey(0.2);
            let col_hands = color::Rgba8Srgb::rgb(124, 124, 170).into();
            let col_secs = color::Rgba8Srgb::rgb(203, 124, 124).into();

            // We use the low-level draw device to draw our clock. This means it is
            // not themeable, but gives us much more flexible draw routines.
            let draw = draw.draw_device();
            // Rust does not (yet!) support downcast to trait-object, thus we must use the shell's
            // DrawPipe (which is otherwise an implementation detail). This supports DrawRounded.
            let mut draw = DrawIface::<DrawPipe<()>>::downcast_from(draw).unwrap();

            let rect = self.core.rect;
            let quad = Quad::conv(rect);
            draw.circle(quad, 0.95, col_face);

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

            draw.text(self.date_pos.cast(), self.date.as_ref(), col_date);
            draw.text(self.time_pos.cast(), self.time.as_ref(), col_time);

            // We use a new pass to control the draw order (force in front).
            let mut draw = draw.new_pass(rect, Offset::ZERO, PassType::Clip);
            let mut line_seg = |t: f32, r1: f32, r2: f32, w, col| {
                let v = Vec2(t.sin(), -t.cos());
                draw.rounded_line(centre + v * r1, centre + v * r2, w, col);
            };

            let secs = self.now.time().num_seconds_from_midnight();
            let a_sec = f32::conv(secs % 60) * (PI / 30.0);
            let a_min = f32::conv(secs % 3600) * (PI / 1800.0);
            let a_hour = f32::conv(secs % 43200) * (PI / (21600.0));

            line_seg(a_hour, 0.0, half * 0.55, half * 0.03, col_hands);
            line_seg(a_min, 0.0, half * 0.8, half * 0.015, col_hands);
            line_seg(a_sec, 0.0, half * 0.9, half * 0.005, col_secs);
        }
    }

    impl Widget for Clock {
        fn configure(&mut self, mgr: &mut ConfigMgr) {
            mgr.request_update(self.id(), 0, Duration::new(0, 0), true);
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::TimerUpdate(0) => {
                    self.now = Local::now();
                    let date = self.now.format("%Y-%m-%d").to_string();
                    let time = self.now.format("%H:%M:%S").to_string();
                    let avail = Size(self.core.rect.size.0, self.core.rect.size.1 / 2);
                    *mgr |= TkAction::REDRAW
                        | set_text_and_prepare(&mut self.date, date, avail)
                        | set_text_and_prepare(&mut self.time, time, avail);
                    let ns = 1_000_000_000 - (self.now.time().nanosecond() % 1_000_000_000);
                    info!("Requesting update in {}ns", ns);
                    mgr.request_update(self.id(), 0, Duration::new(0, ns), true);
                    Response::Used
                }
                _ => Response::Unused,
            }
        }
    }

    impl Clock {
        fn new() -> Self {
            let env = kas::text::Environment {
                align: (Align::Center, Align::Center),
                dpp: 1.0,
                ..Default::default()
            };
            let date = Text::new(env.clone(), "0000-00-00".into());
            let time = Text::new(env, "00:00:00".into());
            Clock {
                core: Default::default(),
                date_pos: Coord::ZERO,
                time_pos: Coord::ZERO,
                now: Local::now(),
                date,
                time,
            }
        }
    }

    impl Window for Self {
        fn title(&self) -> &str { "Clock" }
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let theme = kas::theme::FlatTheme::new();
    kas::shell::Toolkit::new(theme)?.with(Clock::new())?.run()
}
