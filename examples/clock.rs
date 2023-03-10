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
use std::f32::consts::PI;
use std::str::FromStr;
use std::time::Duration;

use kas::draw::color::{Rgba, Rgba8Srgb};
use kas::draw::{Draw, DrawRounded};
use kas::geom::{Offset, Quad, Rect, Vec2};
use kas::prelude::*;
use kas::shell::ShellAssoc;

type Theme = kas::theme::FlatTheme;
type Shell = kas::shell::DefaultShell<Theme>;

impl_scope! {
    #[derive(Clone, Debug)]
    #[widget]
    struct Clock {
        core: widget_core!(),
        date_rect: Rect,
        time_rect: Rect,
        now: DateTime<Local>,
        date: Text<String>,
        time: Text<String>,
    }

    impl Layout for Clock {
        fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            kas::layout::LogicalSize(64.0, 64.0)
                .to_rules_with_factor(axis, mgr.scale_factor(), 3.0)
                .with_stretch(Stretch::High)
        }

        #[inline]
        fn set_rect(&mut self, _: &mut ConfigMgr, rect: Rect) {
            // Force to square
            let size = rect.size.0.min(rect.size.1);
            let size = Size::splat(size);
            let excess = rect.size - size;
            let pos = rect.pos + excess / 2;
            self.core.rect = Rect { pos, size };

            let text_size = Size(size.0, size.1 / 4);
            let text_height = text_size.1 as f32;

            let mut env = self.date.env();
            env.dpem = text_height * 0.5;
            env.bounds = text_size.cast();
            self.date.update_env(env).expect("invalid font_id");
            env.dpem = text_height * 0.7;
            self.time.update_env(env).expect("invalid font_id");

            let time_pos = pos + Offset(0, size.1 * 5 / 8);
            let date_pos = pos + Offset(0, size.1 / 8);
            self.date_rect = Rect::new(date_pos, text_size);
            self.time_rect = Rect::new(time_pos, text_size);
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let accent: Rgba = Rgba8Srgb::from_str("#d7916f").unwrap().into();
            let col_back = Rgba::ga(0.0, 0.5);
            let col_face = accent.multiply(0.4);
            let col_time = Rgba::grey(1.0);
            let col_date = Rgba::grey(0.8);
            let col_hands = accent.multiply(0.7);
            let col_secs = accent;

            // We use the low-level draw device to draw our clock. This means it is
            // not themeable, but gives us much more flexible draw routines.
            let mut draw = draw.draw_iface::<<Shell as ShellAssoc>::DrawShared>().unwrap();

            let rect = self.core.rect;
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

            draw.text(self.date_rect, self.date.as_ref(), col_date);
            draw.text(self.time_rect, self.time.as_ref(), col_time);

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
        fn configure(&mut self, mgr: &mut ConfigCx<Self::Data>) {
            mgr.request_update(self.id(), 0, Duration::new(0, 0), true);
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::TimerUpdate(0) => {
                    self.now = Local::now();
                    let date = self.now.format("%Y-%m-%d").to_string();
                    let time = self.now.format("%H:%M:%S").to_string();
                    self.date
                        .set_and_try_prepare(date)
                        .expect("invalid font_id");
                    self.time
                        .set_and_try_prepare(time)
                        .expect("invalid font_id");
                    let ns = 1_000_000_000 - (self.now.time().nanosecond() % 1_000_000_000);
                    log::info!("Requesting update in {}ns", ns);
                    mgr.request_update(self.id(), 0, Duration::new(0, ns), true);
                    *mgr |= Action::REDRAW;
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
                ..Default::default()
            };
            let date = Text::new_env(env, "0000-00-00".into());
            let time = Text::new_env(env, "00:00:00".into());
            Clock {
                core: Default::default(),
                date_rect: Rect::ZERO,
                time_rect: Rect::ZERO,
                now: Local::now(),
                date,
                time,
            }
        }
    }

    impl Window for Self {
        fn title(&self) -> &str {
            "Clock"
        }

        fn decorations(&self) -> kas::Decorations {
            kas::Decorations::None
        }

        fn transparent(&self) -> bool {
            true
        }
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let theme = Theme::new();
    Shell::new(theme)?.with(Clock::new())?.run()
}
