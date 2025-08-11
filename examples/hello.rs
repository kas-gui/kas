// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Hello world example

use kas::window::Window;
use kas_widgets::{Button, column};

fn main() -> kas::runner::Result<()> {
    let ui = column![
        "Hello, world!",
        Button::label("&Close").with(|cx, _| cx.exit())
    ];
    let window = Window::new(ui, "Hello");

    kas::runner::Runner::new(())?.with(window).run()
}
