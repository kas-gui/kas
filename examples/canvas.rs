// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Canvas example
//!
//! This example is animated. Unfortunately, the Canvas uses tiny-skia for
//! pure-CPU drawing instead of GPU-acceleration, thus performance is poor.

use kas::cast::{Cast, Conv};
use kas::geom::Vec2;
use kas::resvg::{tiny_skia::*, Canvas, CanvasProgram};
use kas::widgets::dialog::Window;
use std::time::Instant;

#[derive(Debug)]
struct Program(Instant);
impl CanvasProgram for Program {
    fn draw(&mut self, pixmap: &mut Pixmap) {
        let size = (200.0, 200.0);
        let scale = Transform::from_scale(
            f32::conv(pixmap.width()) / size.0,
            f32::conv(pixmap.height()) / size.1,
        );

        let paint = Paint {
            shader: LinearGradient::new(
                Point::from_xy(0.0, 0.0),
                Point::from_xy(size.0, size.1),
                vec![
                    GradientStop::new(0.0, Color::BLACK),
                    GradientStop::new(1.0, Color::from_rgba8(0, 255, 200, 255)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            )
            .unwrap(),
            ..Default::default()
        };

        let p = Vec2(110.0, 90.0);
        let t = self.0.elapsed().as_secs_f32();
        let c = t.cos();
        let s = t.sin();

        let mut vv = [
            Vec2(-90.0, -40.0),
            Vec2(-50.0, -30.0),
            Vec2(-30.0, 20.0),
            Vec2(-30.0, -5.0),
            Vec2(-10.0, -30.0),
            Vec2(-50.0, -50.0),
        ];
        for v in &mut vv {
            *v = p + Vec2(c * v.0 - s * v.1, s * v.0 + c * v.1);
        }

        let mut path = PathBuilder::new();
        path.push_circle(p.0 + 10.0, p.1, 100.0);
        path.push_circle(p.0, p.1, 50.0);
        let path = path.finish().unwrap();
        pixmap.fill_path(&path, &paint, FillRule::EvenOdd, scale, None);

        let path = PathBuilder::from_circle(30.0, 180.0, 20.0).unwrap();
        pixmap.fill_path(&path, &paint, FillRule::Winding, scale, None);

        let mut paint = Paint::default();
        paint.set_color_rgba8(230, 90, 50, 255);
        let mut path = PathBuilder::new();
        path.move_to(vv[0].0, vv[0].1);
        path.quad_to(vv[1].0, vv[1].1, vv[2].0, vv[2].1);
        path.quad_to(vv[3].0, vv[3].1, vv[4].0, vv[4].1);
        path.quad_to(vv[5].0, vv[5].1, vv[0].0, vv[0].1);
        let path = path.finish().unwrap();
        pixmap.fill_path(&path, &paint, FillRule::Winding, scale, None);
    }

    fn do_redraw_animate(&mut self) -> (bool, bool) {
        // Set false to disable animation
        (true, true)
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let canvas = Canvas::new(Program(Instant::now())).with_size((400, 400).cast());
    let window = Window::new("Canvas", canvas);

    let theme = kas::theme::FlatTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
