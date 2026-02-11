use kas::prelude::*;
use kas::widgets::SpinBox;
use kas::image::Sprite;

#[derive(Clone, Copy)]
struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl Color {
    fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Color { red, green, blue }
    }
}

#[derive(Debug)]
struct NewBuffer {
    buffer: Vec<u8>,
    size: Size
}

#[derive(Debug)]
struct SetColor(bool, ColorComponent);

#[derive(Debug)]
enum ColorComponent {
    Red(u8),
    Green(u8),
    Blue(u8)
}

fn lerp(start: u8, end: u8, i: usize, max: usize) -> u8 {
    let t = (i as f64) / (max as f64);
    let exact = (1. - t) * start as f64 + t * end as f64;
    exact as u8
}

async fn render_gradient(color1: Color, color2: Color, size: Size) -> NewBuffer {
    let width = size.0 as usize;
    let height = size.1 as usize;
    let mut buffer = vec![255; 4 * width * height];

    for y in 0..height {
        let r = lerp(color1.red, color2.red, y, height);
        let g = lerp(color1.green, color2.green, y, height);
        let b = lerp(color1.blue, color2.blue, y, height);
        for x in 0..width {
            let offset = y * width + x;
            buffer[4 * offset + 0] = r;
            buffer[4 * offset + 1] = g;
            buffer[4 * offset + 2] = b;
        }
    }

    NewBuffer { buffer, size }
}

#[impl_self]
mod Gradient {
    #[widget]
    #[layout(column![
        row!["R", self.red1, "G", self.green1, "B", self.blue1],
        self.sprite.with_stretch(Stretch::Maximize, Stretch::Maximize),
        row!["R", self.red2, "G", self.green2, "B", self.blue2],
    ])]
    struct Gradient {
        core: widget_core!(),
        color1: Color,
        color2: Color,
        #[widget(&self.color1.red)]
        red1: SpinBox<u8, u8>,
        #[widget(&self.color1.green)]
        green1: SpinBox<u8, u8>,
        #[widget(&self.color1.blue)]
        blue1: SpinBox<u8, u8>,
        #[widget(&self.color2.red)]
        red2: SpinBox<u8, u8>,
        #[widget(&self.color2.green)]
        green2: SpinBox<u8, u8>,
        #[widget(&self.color2.blue)]
        blue2: SpinBox<u8, u8>,
        #[widget]
        sprite: Sprite,
        rendering: bool
    }

    impl Self {
        fn new(size: impl Into<kas::layout::LogicalSize>) -> Self {
            use ColorComponent::*;

            Gradient {
                core: Default::default(),
                color1: Color::rgb(255, 0, 0),
                color2: Color::rgb(0, 0, 255),
                red1: SpinBox::new_msg(0..=255, |_, val| *val, |val| SetColor(true, Red(val))),
                green1: SpinBox::new_msg(0..=255, |_, val| *val, |val| SetColor(true, Green(val))),
                blue1: SpinBox::new_msg(0..=255, |_, val| *val, |val| SetColor(true, Blue(val))),
                red2: SpinBox::new_msg(0..=255, |_, val| *val, |val| SetColor(false, Red(val))),
                green2: SpinBox::new_msg(0..=255, |_, val| *val, |val| SetColor(false, Green(val))),
                blue2: SpinBox::new_msg(0..=255, |_, val| *val, |val| SetColor(false, Blue(val))),
                sprite: Sprite::new()
                    .with_logical_size(size)
                    .with_stretch(Stretch::Maximize)
                    .with_fixed_aspect_ratio(false),
                rendering: false,
            }
        }

        fn rerender(&mut self, event: &mut EventState) {
            event.send_spawn(
                self.id(),
                render_gradient(self.color1, self.color2, self.sprite.rect().size)
            );
            self.rendering = true;
        }
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut SizeCx<'_>, rect: Rect, hints: AlignHints) {
            kas::MacroDefinedLayout::set_rect(self, cx, rect, hints);
            if self.sprite.image_size() != self.sprite.rect().size && !self.rendering {
                self.rerender(cx);
            }
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, cx: &mut EventCx, _: &()) {
            if let Some(NewBuffer { buffer, size }) = cx.try_pop() {
                self.rendering = false;

                let draw = cx.draw_shared();
                if let Some(handle) = self.sprite.handle()
                    && draw.image_size(handle) == Some(size)
                {
                    if let Ok(action) = draw.image_upload(
                        handle,
                        &buffer[..]
                    ) {
                        cx.action_redraw(action);
                    }
                } else {
                    if let Ok(handle) = draw.image_alloc(kas::draw::ImageFormat::Rgba8, size) {
                        if let Ok(action) = draw.image_upload(
                            &handle,
                            &buffer[..]
                        ) {
                            self.sprite.set(cx, handle);
                            cx.action_redraw(action);
                        }
                    }
                }

                if self.sprite.image_size() != self.sprite.rect().size {
                    self.rerender(cx);
                }
            }

            if let Some(SetColor(first, component)) = cx.try_pop() {
                use ColorComponent::*;

                let c = if first { &mut self.color1 } else { &mut self.color2 };
                match component {
                    Red(r) => c.red = r,
                    Green(g) => c.green = g,
                    Blue(b) => c.blue = b
                }

                cx.update(self.as_node(&()));
                self.rerender(cx);
            }
        }
    }
}

fn main() -> kas::runner::Result<()> {
    kas::runner::Runner::new(())
        .unwrap()
        .with(Window::new(Gradient::new((500., 500.)), "Gradient"))
        .run()
}
