// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! File system explorer

mod viewer;

use kas::prelude::*;
use kas::window::Window;
use std::path::PathBuf;

fn main() -> kas::runner::Result<()> {
    let ui = viewer::viewer().with_state(PathBuf::from("."));
    let window = Window::new(ui, "File System Explorer").escapable();

    kas::runner::Runner::new(())?.with(window).run()
}
