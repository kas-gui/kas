// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A counter synchronised between multiple windows

use kas::widget::Spinner;
use kas::{Action, ErasedStack, Window};

#[derive(Clone, Debug)]
struct Set(i32);

struct Data {
    count: i32,
}
impl kas::AppData for Data {
    fn handle_messages(&mut self, messages: &mut ErasedStack) -> Action {
        if let Some(Set(count)) = messages.try_pop() {
            self.count = count;
            println!("count: {}", self.count);
            Action::UPDATE
        } else {
            Action::EMPTY
        }
    }
}

fn counter(title: &str) -> Window<Data> {
    let s = Spinner::new_msg(i32::MIN..=i32::MAX, |data: &Data| data.count, Set);
    Window::new(s, title)
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let data = Data { count: 0 };
    let theme = kas_wgpu::ShadedTheme::new().with_font_size(24.0);

    kas::shell::DefaultShell::new(data, theme)?
        .with(counter("Counter 1"))?
        .with(counter("Counter 2"))?
        .run()
}
