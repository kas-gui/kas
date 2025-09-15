// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple text editor

use kas::prelude::*;
use kas::widgets::{Button, EditBox, EditGuard, Filler, column, row};

#[autoimpl(Clone, Debug)]
enum EditorAction {
    New,
    Open,
    Save,
    SaveAs,
}

fn menus() -> impl Widget<Data = ()> {
    row![
        Button::label_msg("&New", EditorAction::New),
        Button::label_msg("&Open", EditorAction::Open),
        Button::label_msg("&Save", EditorAction::Save),
        Button::label_msg("Save&As", EditorAction::SaveAs),
        Filler::low()
    ]
}

struct Guard;
impl EditGuard for Guard {
    type Data = ();
}

#[impl_self]
mod Editor {
    #[widget]
    #[layout(self.editor)]
    struct Editor {
        core: widget_core!(),
        #[widget]
        editor: EditBox<Guard>,
    }

    impl Events for Self {
        type Data = ();
    }

    impl Self {
        fn new() -> Self {
            Editor {
                core: Default::default(),
                editor: EditBox::new(Guard)
                    .with_multi_line(true)
                    .with_lines(5.0, 20.0)
                    .with_width_em(10.0, 30.0),
            }
        }
    }
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let theme = kas::theme::FlatTheme::new();
    let app = kas::runner::Runner::with_theme(theme).build(())?;

    let ui = column![menus(), Editor::new()];
    let window = Window::new(ui, "Editor");
    app.with(window).run()
}
