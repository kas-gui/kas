// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A counter synchronised between multiple windows
//!
//! Each window shares the counter, but has its own increment step.

use kas::widget::{format_data, label, Adapt, Button, Slider};
use kas::{Action, ErasedStack, Window};

#[derive(Clone, Debug)]
struct Increment(i32);

#[derive(Clone, Copy, Debug)]
struct Count(i32);
impl kas::AppData for Count {
    fn handle_messages(&mut self, messages: &mut ErasedStack) -> Action {
        if let Some(Increment(add)) = messages.try_pop() {
            self.0 += add;
            Action::UPDATE
        } else {
            Action::empty()
        }
    }
}

fn counter(title: &str) -> Window<Count> {
    // Per window state: (count, step).
    // We must store a local copy of the count in order to have a Data instance
    // to pass by reference.
    // (Eventually we may be able to support Adapt forwarding data by reference,
    // but this would require Rust to support object-safe GATs.)
    type Data = (Count, i32);
    // Initial count is replaced during configure, but initial step is used.
    let initial: Data = (Count(0), 1);

    #[derive(Clone, Debug)]
    struct SetValue(i32);

    // let slider = Slider::<_, _>::new_msg(1..=10, |data: &Data| data.1, SetValue);
    let slider = Slider::right(1..=10, |_, data: &Data| data.1).with_msg(SetValue);
    let ui = kas::column![
        format_data!(data: &Data, "Count: {}", data.0.0),
        row![slider, format_data!(data: &Data, "{}", data.1)],
        row![
            Button::new(label("Sub")).with(|cx, data: &Data| cx.push(Increment(-data.1))),
            Button::new(label("Add")).with(|cx, data: &Data| cx.push(Increment(data.1))),
        ],
    ];

    let ui = Adapt::new(ui, initial)
        .on_update(|_, count, state| state.0 = *count)
        .on_message(|_, state, SetValue(v)| state.1 = v);
    Window::new(ui, title)
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let count = Count(0);
    let theme = kas_wgpu::ShadedTheme::new().with_font_size(24.0);

    kas::shell::DefaultShell::new(count, theme)?
        .with(counter("Counter 1"))?
        .with(counter("Counter 2"))?
        .run()
}
