//! Main view of a directory

use kas::prelude::*;
use kas::view::{ListView, clerk, driver::View};
use std::io;
use std::{ops::Range, path::PathBuf};

#[derive(Debug)]
struct Entry {
    display: String,
    entry: Result<PathBuf, io::Error>,
}

#[derive(Debug)]
struct NewEntries(Vec<Entry>);

#[derive(Default)]
struct Clerk {
    path: PathBuf,
    entries: Vec<Entry>,
}

impl clerk::Clerk<usize> for Clerk {
    type Data = PathBuf;
    type Item = String;

    fn len(&self, _: &Self::Data, _: usize) -> clerk::Len<usize> {
        clerk::Len::Known(self.entries.len())
    }
}

impl clerk::AsyncClerk<usize> for Clerk {
    type Key = usize;

    fn update(
        &mut self,
        cx: &mut ConfigCx<'_>,
        id: Id,
        _: Range<usize>,
        path: &Self::Data,
    ) -> clerk::Changes<usize> {
        if *path != self.path {
            self.path = path.clone();

            let path = path.clone();
            cx.send_spawn(id, async move {
                let dirs = std::fs::read_dir(&path).expect("failed to read {path}");
                NewEntries(
                    dirs.map(|entry| {
                        let entry = entry.map(|entry| entry.path());
                        let display = match &entry {
                            Ok(path) => format!("{}", path.display()),
                            Err(err) => format!("Error: {err}"),
                        };
                        Entry { display, entry }
                    })
                    .collect(),
                )
            });

            clerk::Changes::Any
        } else {
            clerk::Changes::None
        }
    }

    fn handle_messages(
        &mut self,
        cx: &mut EventCx<'_>,
        _: Id,
        _: Range<usize>,
        _: &Self::Data,
        _: Option<Self::Key>,
    ) -> clerk::Changes<usize> {
        if let Some(NewEntries(entries)) = cx.try_pop() {
            self.entries = entries;
            clerk::Changes::Any
        } else {
            clerk::Changes::None
        }
    }
}

impl clerk::TokenClerk<usize> for Clerk {
    type Token = usize;

    fn update_token(
        &self,
        _: &Self::Data,
        index: usize,
        update_item: bool,
        token: &mut Option<Self::Token>,
    ) -> clerk::TokenChanges {
        let expected = (index < self.entries.len()).then_some(index);
        clerk::update_token(expected, update_item, token)
    }

    fn item<'r>(&'r self, _: &'r Self::Data, token: &'r Self::Token) -> &'r Self::Item {
        &self
            .entries
            .get(*token)
            .expect("bad token or missing entry")
            .display
    }
}

#[impl_self]
mod DirView {
    #[widget]
    #[layout(self.list)]
    #[derive(Default)]
    pub struct DirView {
        core: widget_core!(),
        #[widget]
        list: ListView<Clerk, View, kas::dir::Down>,
    }

    impl Events for Self {
        type Data = PathBuf;
    }
}
