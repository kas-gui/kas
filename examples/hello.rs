// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Hello world example

use kas::widget::dialog::MessageBox;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = MessageBox::new("Message").into_window("Hello world");

    let theme = kas::theme::FlatTheme::new();
    kas::shell::DefaultShell::new((), theme)?
        .with(window)?
        .run()
}
