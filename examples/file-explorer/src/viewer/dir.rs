//! Main view of a directory

use crate::{Entry, report_io_error};
use kas::prelude::*;
use kas::view::{ListView, clerk};
use std::{ops::Range, path::PathBuf};

#[derive(Debug)]
struct NewEntries(Vec<Entry>);

#[derive(Default)]
struct Clerk {
    path: PathBuf,
    entries: Vec<Entry>,
}

impl clerk::Clerk<usize> for Clerk {
    type Data = PathBuf;
    type Item = Entry;

    fn len(&self, _: &Self::Data, _: usize) -> clerk::Len<usize> {
        clerk::Len::Known(self.entries.len())
    }

    fn mock_item(&self, _: &Self::Data) -> Option<Entry> {
        Some(PathBuf::new())
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
            log::trace!("update: path=\"{}\"", path.display());
            self.path = path.clone();

            let path = path.clone();
            cx.send_spawn(id, async move {
                match std::fs::read_dir(&path) {
                    Ok(dirs) => NewEntries(
                        dirs.filter_map(|entry| match entry {
                            Ok(entry) => Some(entry.path()),
                            Err(err) => {
                                report_io_error(&path, err);
                                None
                            }
                        })
                        .filter(|path| {
                            if let Some(name) = path.file_name()
                                && name.as_encoded_bytes().get(0) == Some(&b'.')
                            {
                                false
                            } else {
                                true
                            }
                        })
                        .collect(),
                    ),
                    Err(err) => {
                        report_io_error(&path, err);
                        NewEntries(vec![])
                    }
                }
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
            log::trace!("handle_messages: NewEntries{entries:?}");
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
        list: ListView<Clerk, crate::tile::Driver, kas::dir::Down>,
    }

    impl Events for Self {
        type Data = PathBuf;
    }
}
