// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Hello world example

use kas::widgets::dialog::MessageBox;

fn main() -> kas::runner::Result<()> {
    let window = MessageBox::new("Message").into_window("Hello world");

    kas::runner::Runner::new(())?.with(window).run()
}
