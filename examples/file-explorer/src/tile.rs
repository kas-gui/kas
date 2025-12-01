use crate::Entry;
use kas::prelude::*;
use kas::widgets::{Page, Stack, Text};

fn generic() -> impl Widget<Data = Entry> {
    Text::new_gen(|_, entry: &Entry| match &entry {
        Ok(path) => format!("{}", path.display()),
        Err(err) => format!("Error: {err}"),
    })
}

type Tile = Stack<Entry>;

fn tile() -> Tile {
    Stack::from([Page::new(generic())])
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
