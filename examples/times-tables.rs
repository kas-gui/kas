//! Do you know your times tables?

use kas::prelude::*;
use kas::view::{DataClerk, GridIndex, GridView, SelectionMode, SelectionMsg, driver};
use kas::widgets::{EditBox, ScrollBars, column, row};
use std::ops::Range;

/// A cache of the visible part of our table
#[derive(Debug, Default)]
struct TableCache {
    dim: u32,
    col_len: usize,
    col_start: u32,
    row_start: u32,
    contents: Vec<u64>,
}

fn product(x: u32, y: u32) -> u64 {
    let x = u64::conv(x + 1);
    let y = u64::conv(y + 1);
    x * y
}

impl DataClerk<GridIndex> for TableCache {
    /// Our table is square; it's size is input.
    type Data = u32;

    /// We re-usize the index as our key.
    type Key = GridIndex;

    /// Data items are `u64` since e.g. 65536² is not representable by `u32`.
    type Item = u64;

    fn update(&mut self, _: &mut ConfigCx, _: Id, dim: &Self::Data) {
        self.dim = *dim;
    }

    fn len(&self, _: &Self::Data) -> GridIndex {
        GridIndex::splat(self.dim)
    }

    fn prepare_range(&mut self, _: &mut ConfigCx, _: Id, _: &Self::Data, range: Range<GridIndex>) {
        // This is a simple hack to cache contents for the given range for usage by item()
        let x_len = usize::conv(range.end.col - range.start.col);
        let y_len = usize::conv(range.end.row - range.start.row);
        if x_len != self.col_len
            || x_len * y_len != self.contents.len()
            || self.col_start != range.start.col
            || self.row_start != range.start.row
        {
            self.col_len = x_len;
            self.col_start = range.start.col;
            self.row_start = range.start.row;
            self.contents.clear();
            self.contents.reserve(x_len * y_len);

            for y in range.start.row..range.end.row {
                for x in range.start.col..range.end.col {
                    self.contents.push(product(x, y));
                }
            }
        }
    }

    fn key(&self, _: &Self::Data, index: GridIndex) -> Option<Self::Key> {
        Some(index)
    }

    fn item(&self, _: &Self::Data, key: &Self::Key) -> Option<&Self::Item> {
        // We are required to return a reference, otherwise we would simply
        // calculate the value here!
        let GridIndex { col, row } = *key;
        let xrel = usize::conv(col - self.col_start);
        let yrel = usize::conv(row - self.row_start);
        let i = xrel + yrel * self.col_len;
        self.contents.get(i)
    }
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let table = GridView::new(TableCache::default(), driver::NavView)
        .with_num_visible(12, 12)
        .with_selection_mode(SelectionMode::Single);
    let table = ScrollBars::new(table);

    #[derive(Debug)]
    struct SetLen(u32);

    let ui = column![
        row!["From 1 to", EditBox::parser(|dim: &u32| *dim, SetLen)],
        table.align(AlignHints::RIGHT),
    ];
    let ui = ui
        .with_state(12)
        .on_message(|_, dim, SetLen(len)| *dim = len)
        .on_message(|_, _, selection| match selection {
            SelectionMsg::<GridIndex>::Select(GridIndex { col, row }) => {
                println!("{} × {} = {}", col + 1, row + 1, product(col, row));
            }
            _ => (),
        });
    let window = Window::new(ui, "Times-Tables");

    let theme = kas::theme::SimpleTheme::new();
    kas::runner::Runner::with_theme(theme)
        .build(())?
        .with(window)
        .run()
}
