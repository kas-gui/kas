// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple text editor

use kas::prelude::*;
use kas::widgets::{Button, EditBox, EditField, EditGuard, Filler, column, dialog, row};
use rfd::FileHandle;
use std::error::Error;

#[autoimpl(Clone, Debug, PartialEq, Eq)]
enum EditorAction {
    New,
    Open,
    Save,
    SaveAs,
}

#[derive(Debug)]
struct OpenFile(Option<FileHandle>);

#[derive(Debug)]
struct SaveFile(Option<FileHandle>);

#[derive(Debug)]
struct SetContents(Option<String>);

#[derive(Debug)]
struct Saved(std::io::Result<()>);

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
        pending: Option<EditorAction>,
        file: Option<FileHandle>,
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.set_send_target_for::<EditorAction>(self.id());
        }

        fn handle_messages(&mut self, cx: &mut EventCx<'_>, _: &()) {
            if let Some(action) = cx.try_pop() {
                if self.editor.guard().edited
                    && matches!(action, EditorAction::New | EditorAction::Open)
                {
                    self.pending = Some(action);
                    dialog::AlertUnsaved::new("The document has been modified. Do you want to save or discard the changes?")
                        .with_title("Open document")
                        .display_for(cx, self.id());
                    return;
                }

                self.do_action(cx, action);
            } else if let Some(result) = cx.try_pop() {
                // Handle the result of AlertUnsaved dialog:
                match result {
                    dialog::UnsavedResult::Save => {
                        // self.pending will be handled by Saved handler
                        self.do_action(cx, EditorAction::Save);
                        return;
                    }
                    dialog::UnsavedResult::Discard => (),
                    dialog::UnsavedResult::Cancel => {
                        self.pending = None;
                        return;
                    }
                }

                if let Some(action) = self.pending.take() {
                    self.do_action(cx, action);
                }
            } else if let Some(OpenFile(file)) = cx.try_pop() {
                // Assume that no actions handled since the open was requested
                self.file = file.clone();
                if let Some(file) = file {
                    cx.send_async(self.id(), async move {
                        let contents = file.read().await;
                        match String::from_utf8(contents) {
                            Ok(text) => SetContents(Some(text)),
                            Err(err) => {
                                // TODO: display error in UI
                                log::warn!("Input is invalid UTF-8: {err}");
                                let mut source = err.source();
                                while let Some(err) = source {
                                    log::warn!("Cause: {err}");
                                    source = err.source();
                                }
                                SetContents(None)
                            }
                        }
                    });
                }
            } else if let Some(SaveFile(file)) = cx.try_pop() {
                self.file = file;
                self.do_action(cx, EditorAction::Save);
            } else if let Some(SetContents(Some(text))) = cx.try_pop() {
                self.editor.set_string(cx, text);
                self.editor.guard_mut().edited = false;
            } else if let Some(Saved(result)) = cx.try_pop() {
                match result {
                    Ok(()) => self.editor.guard_mut().edited = false,
                    Err(err) => {
                        // TODO: display error in UI
                        log::warn!("File save error: {err}");
                        let mut source = err.source();
                        while let Some(err) = source {
                            log::warn!("Cause: {err}");
                            source = err.source();
                        }
                    }
                }

                if let Some(action) = self.pending.take() {
                    self.do_action(cx, action);
                }
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
                pending: None,
                file: None,
            }
        }

        fn do_action(&mut self, cx: &mut EventCx<'_>, action: EditorAction) {
            match action {
                EditorAction::New => {
                    self.editor.set_string(cx, String::new());
                    self.editor.guard_mut().edited = false;
                    self.file = None;
                }
                EditorAction::Open => {
                    let mut picker = rfd::AsyncFileDialog::new()
                        .add_filter("Plain text", &["txt"])
                        .set_title("Open file");
                    if let Some(window) = cx.winit_window() {
                        picker = picker.set_parent(window);
                    }
                    cx.send_async(self.id(), async { OpenFile(picker.pick_file().await) });
                }
                EditorAction::Save | EditorAction::SaveAs => {
                    if action == EditorAction::Save
                        && let Some(file) = self.file.clone()
                    {
                        let contents = self.editor.clone_string();
                        cx.send_async(self.id(), async move {
                            Saved(file.write(contents.as_str().as_bytes()).await)
                        });
                    } else {
                        let mut picker = rfd::AsyncFileDialog::new()
                            .add_filter("Plain text", &["txt"])
                            .set_title("Save file");
                        if let Some(window) = cx.winit_window() {
                            picker = picker.set_parent(window);
                        }
                        cx.send_async(self.id(), async { SaveFile(picker.save_file().await) });
                    }
                }
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
