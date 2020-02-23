// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Clock example
#![feature(proc_macro_hygiene)]

extern crate chrono;

use chrono::prelude::*;
use log::info;
use std::f32::consts::PI;
use std::time::Duration;

use kas::draw::{Colour, DrawHandle, DrawRounded, DrawText, SizeHandle, TextClass, TextProperties};
use kas::event::{Manager, ManagerState};
use kas::geom::{Coord, Rect, Size};
use kas::layout::{AxisInfo, SizeRules};
use kas::widget::Window;
use kas::{Align, AlignHints, Direction, Layout, ThemeApi, Widget, WidgetCore};
use kas_wgpu::draw::DrawPipe;

#[handler]
#[derive(Clone, Debug, kas :: macros :: Widget)]
struct Clock {
    #[core]
    core: kas::CoreData,
    date_rect: Rect,
    time_rect: Rect,
    font_scale: f32,
    now: DateTime<Local>,
    date: String,
    time: String,
}

impl Layout for Clock {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, _: AxisInfo) -> SizeRules {
        // Always use value for horiz axis: we want a square shape
        let axis = AxisInfo::new(Direction::Horizontal, None);
        size_handle.text_bound("0000-00-00", TextClass::Label, axis)
    }

    #[inline]
    fn set_rect(&mut self, _size_handle: &mut dyn SizeHandle, rect: Rect, _align: AlignHints) {
        // Force to square
        let size = rect.size.0.min(rect.size.1);
        let size = Size::uniform(size);
        let excess = rect.size - size;
        let pos = rect.pos + (excess * 0.5);
        self.core.rect = Rect { pos, size };

        let half_size = Size(size.0, size.1 / 2);
        self.date_rect = Rect::new(pos + Size(0, size.1 - half_size.1), half_size);
        self.time_rect = Rect::new(pos, half_size);
        self.font_scale = size.1 as f32 * 0.125;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState) {
        let col_face = Colour::grey(0.4);
        let col_hands = Colour::new(0.2, 0.2, 0.4);
        let col_secs = Colour::new(0.6, 0.2, 0.2);
        let col_text = Colour::grey(0.0);

        // We use the low-level draw device to draw our clock. This means it is
        // not themeable, but gives us much more flexible draw routines.
        //
        // Note: offset is used for scroll-regions, and should be zero here;
        // we add it anyway as is recommended.
        let (region, offset, draw) = draw_handle.draw_device();
        let draw = draw.as_any_mut().downcast_mut::<DrawPipe>().unwrap();

        draw.circle(region, self.core.rect + offset, 0.95, col_face);

        let half = (self.core.rect.size.1 / 2) as i32;
        let c = self.core.rect.pos + offset + Coord(half, half);

        let mut line_seg = |t: f32, r1: f32, r2: f32, w, col| {
            let (sin, cos) = (t.sin(), -t.cos());
            let p1 = Coord((r1 * sin).round() as i32, (r1 * cos).round() as i32);
            let p2 = Coord((r2 * sin).round() as i32, (r2 * cos).round() as i32);
            draw.rounded_line(region, c + p1, c + p2, w, col);
        };

        let half = half as f32;
        let w = half * 0.015625;
        let l = w * 5.0;
        let r = half - w;
        for d in 0..12 {
            let l = if d % 3 == 0 { 2.0 * l } else { l };
            line_seg(d as f32 * (PI / 6.0), r - l, r, w, col_face);
        }

        let secs = self.now.time().num_seconds_from_midnight();
        line_seg(
            (secs % (12 * 3600)) as f32 * (PI / (6.0 * 3600.0)),
            0.0,
            half * 0.55,
            half * 0.03,
            col_hands,
        );
        line_seg(
            (secs % 3600) as f32 * (PI / 1800.0),
            0.0,
            half * 0.8,
            half * 0.015,
            col_hands,
        );
        line_seg(
            (secs % 60) as f32 * (PI / 30.0),
            0.0,
            half * 0.9,
            half * 0.005,
            col_secs,
        );

        let props = TextProperties {
            scale: self.font_scale,
            col: col_text,
            align: (Align::Centre, Align::Centre),
            ..TextProperties::default()
        };
        draw.text(self.date_rect + offset, &self.date, props);
        draw.text(self.time_rect + offset, &self.time, props);
    }
}

impl Widget for Clock {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.update_on_timer(Duration::new(0, 0), self.id());
    }

    fn update_timer(&mut self, mgr: &mut Manager) -> Option<Duration> {
        self.now = Local::now();
        mgr.redraw(self.id());
        self.date = self.now.format("%Y-%m-%d").to_string();
        self.time = self.now.format("%H:%M:%S").to_string();
        let ns = 1_000_000_000 - (self.now.time().nanosecond() % 1_000_000_000);
        info!("Requesting update in {}ns", ns);
        Some(Duration::new(0, ns))
    }
}

impl Clock {
    fn new() -> Self {
        Clock {
            core: Default::default(),
            date_rect: Rect::default(),
            time_rect: Rect::default(),
            font_scale: 0.0,
            now: Local::now(),
            date: "".to_string(),
            time: "".to_string(),
        }
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let window = Window::new("Clock", Clock::new());

    let mut theme = kas_theme::FlatTheme::new();
    theme.set_font_size(32.0);
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
