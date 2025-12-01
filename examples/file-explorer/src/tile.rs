use crate::Entry;
use kas::prelude::*;
use kas::widgets::{Adapt, Page, Stack, Text};
use std::borrow::Cow;
use std::path::PathBuf;

#[autoimpl(Debug)]
pub enum State {
    Initial,
    Error(String),
    Unknown(PathBuf),
}

impl State {
    fn update(&mut self, entry: &Entry) {
        let Ok(path) = entry else {
            *self = State::Error(format!("Error: {:?}", entry.as_ref().unwrap_err()));
            return;
        };

        if matches!(self, State::Unknown(p) if p == path) {
            return;
        }

        *self = State::Unknown(path.clone());
    }
}

fn generic() -> impl Widget<Data = State> {
    Text::new_update(|_, entry: &State, text: &mut String| {
        let new_text: Cow<str> = match &entry {
            State::Initial => Cow::from("loading"),
            State::Error(text) => text.into(),
            State::Unknown(path) => format!("{}", path.display()).into(),
        };
        if *text != new_text {
            *text = new_text.into_owned();
            true
        } else {
            false
        }
    })
}

type Tile = Adapt<Entry, Stack<State>>;

fn tile() -> Tile {
    Stack::from([Page::new(generic())])
        .with_state(State::Initial)
        .on_update(|_, state, entry| {
            state.update(entry);
        })
}

#[derive(Default)]
pub struct Driver;

impl kas::view::Driver<usize, Entry> for Driver {
    const TAB_NAVIGABLE: bool = false;
    type Widget = Tile;

    fn make(&mut self, _: &usize) -> Tile {
        tile()
    }

    fn navigable(_: &Tile) -> bool {
        false
    }
}
