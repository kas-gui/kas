// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Canvas example

use kas::cast::Conv;
use kas::geom::Size;
use kas::widget::tiny_skia::*;
use kas::widget::{Canvas, CanvasDrawable, Window};

#[derive(Debug)]
struct Program;
impl CanvasDrawable for Program {
    fn draw(&self, pixmap: &mut Pixmap) {
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

        let mut path = PathBuilder::new();
        path.push_circle(120.0, 90.0, 100.0);
        path.push_circle(110.0, 90.0, 50.0);
        let path = path.finish().unwrap();
        pixmap.fill_path(&path, &paint, FillRule::EvenOdd, scale, None);

        let path = PathBuilder::from_circle(30.0, 180.0, 20.0).unwrap();
        pixmap.fill_path(&path, &paint, FillRule::Winding, scale, None);

        let mut paint = Paint::default();
        paint.set_color_rgba8(230, 90, 50, 255);
        let mut path = PathBuilder::new();
        path.move_to(20.0, 50.0);
        path.quad_to(60.0, 60.0, 80.0, 110.0);
        path.quad_to(80.0, 85.0, 100.0, 60.0);
        path.quad_to(60.0, 40.0, 20.0, 50.0);
        let path = path.finish().unwrap();
        pixmap.fill_path(&path, &paint, FillRule::Winding, scale, None);
    }
}

fn main() -> Result<(), kas::shell::Error> {
    env_logger::init();

    let canvas = Canvas::new(Program, Size(400, 400));
    let window = Window::new("Canvas", canvas);

    let theme = kas::theme::FlatTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
