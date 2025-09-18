// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple text editor

use kas::prelude::*;
use kas::widgets::dialog::MessageBox;
use kas::widgets::{Button, EditBox, EditField, EditGuard, Filler, column, row};
use std::error::Error;

#[autoimpl(Clone, Debug)]
enum EditorAction {
    New,
    Open,
    Save,
    SaveAs,
}

#[derive(Debug)]
struct OpenFile(Option<String>);

fn menus() -> impl Widget<Data = ()> {
    row![
        Button::label_msg("&New", EditorAction::New),
        Button::label_msg("&Open", EditorAction::Open),
        Button::label_msg("&Save", EditorAction::Save),
        Button::label_msg("Save&As", EditorAction::SaveAs),
        Filler::low()
    ]
}

#[derive(Default)]
struct Guard {
    edited: bool,
}
impl EditGuard for Guard {
    type Data = ();

    fn edit(edit: &mut EditField<Self>, _: &mut EventCx<'_>, _: &Self::Data) {
        edit.guard.edited = true;
    }
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

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.set_send_target_for::<EditorAction>(self.id());
        }

        fn handle_messages(&mut self, cx: &mut EventCx<'_>, _: &()) {
            if let Some(msg) = cx.try_pop() {
                if self.editor.guard().edited
                    && matches!(msg, EditorAction::New | EditorAction::Open)
                {
                    MessageBox::new("Refusing operation due to edited contents")
                        .display(cx, "Open document");
                    return;
                }

                match msg {
                    EditorAction::New => {
                        self.editor.set_string(cx, String::new());
                        self.editor.guard_mut().edited = false;
                    }
                    EditorAction::Open => {
                        let mut picker = rfd::AsyncFileDialog::new()
                            .add_filter("Plain text", &["txt"])
                            .set_title("Open file");
                        if let Some(window) = cx.winit_window() {
                            picker = picker.set_parent(window);
                        }
                        cx.send_async(self.id(), async {
                            let Some(file) = picker.pick_file().await else {
                                return OpenFile(None);
                            };

                            let contents = file.read().await;
                            match String::from_utf8(contents) {
                                Ok(text) => OpenFile(Some(text)),
                                Err(err) => {
                                    // TODO: display error in UI
                                    log::warn!("Input is invalid UTF-8: {err}");
                                    let mut source = err.source();
                                    while let Some(err) = source {
                                        log::warn!("Cause: {err}");
                                        source = err.source();
                                    }
                                    OpenFile(None)
                                }
                            }
                        });
                    }
                    _ => todo!(),
                }
            } else if let Some(OpenFile(Some(text))) = cx.try_pop() {
                self.editor.set_string(cx, text);
                self.editor.guard_mut().edited = false;
            }
        }
    }

    impl Self {
        fn new() -> Self {
            Editor {
                core: Default::default(),
                editor: EditBox::new(Guard::default())
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
