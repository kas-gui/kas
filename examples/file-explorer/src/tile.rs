use crate::{Entry, report_io_error};
use kas::Tile as _;
use kas::prelude::*;
use kas::widgets::{Button, Page, Stack, Text};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

#[autoimpl(Debug)]
pub enum State {
    Initial,
    Error,
    Unknown(PathBuf),
    Directory(PathBuf, String),
}

impl State {
    fn update(&mut self, entry: &Entry) -> Option<PathBuf> {
        log::trace!("State::update: {entry:?}");
        let Ok(path) = entry else {
            *self = State::Error;
            return None;
        };

        if path.as_os_str().is_empty() {
            *self = State::Initial;
            None
        } else if !matches!(self, State::Unknown(p) if p == path) {
            *self = State::Unknown(path.clone());
            Some(path.clone())
        } else {
            None
        }
    }
}

fn generic() -> impl Widget<Data = State> {
    Text::new_update(|_, entry: &State, text: &mut String| {
        let new_text: Cow<str> = match &entry {
            State::Initial => Cow::from("loading"),
            State::Error => "<error>".into(),
            State::Unknown(path) => format!("{}", path.display()).into(),
            _ => "<bad state>".into(),
        };
        if *text != new_text {
            *text = new_text.into_owned();
            true
        } else {
            false
        }
    })
}

fn directory() -> impl Widget<Data = State> {
    Button::new(Text::new_str(|state: &State| match state {
        State::Directory(_, name) => name,
        _ => "<bad state>",
    }))
}

#[impl_self]
mod Tile {
    #[widget]
    #[layout(self.stack)]
    pub struct Tile {
        core: widget_core!(),
        state: State,
        #[widget(&self.state)]
        stack: Stack<State>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let rules = cx.logical(128.0, 128.0).with_margin(16.0).build(axis);
            self.stack.size_rules(cx, axis).max(rules)
        }
    }

    impl Events for Self {
        type Data = Entry;

        fn update(&mut self, cx: &mut ConfigCx, entry: &Entry) {
            if let Some(path) = self.state.update(entry) {
                cx.send_spawn(self.id(), async {
                    let md = match fs::metadata(&path) {
                        Ok(md) => md,
                        Err(err) => {
                            report_io_error(&path, err);
                            return State::Error;
                        }
                    };

                    if md.is_dir() {
                        let name = path
                            .file_name()
                            .map(|os_str| os_str.to_string_lossy().to_string())
                            .unwrap_or_default();
                        State::Directory(path, name)
                    } else {
                        State::Unknown(path)
                    }
                });
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(state) = cx.try_pop() {
                self.state = state;
                let page = match &self.state {
                    State::Directory(_, _) => 1,
                    _ => 0,
                };
                self.stack.set_active(cx, &self.state, page);
            }
        }
    }

    impl Self {
        fn new() -> Self {
            Tile {
                core: Default::default(),
                state: State::Initial,
                stack: Stack::from([Page::new(generic()), Page::new(directory())]),
            }
        }
    }
}

#[derive(Default)]
pub struct Driver;

impl kas::view::Driver<usize, Entry> for Driver {
    const TAB_NAVIGABLE: bool = false;
    type Widget = Tile;

    fn make(&mut self, _: &usize) -> Tile {
        Tile::new()
    }

    fn navigable(_: &Tile) -> bool {
        false
    }
}
