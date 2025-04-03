//! Do you know your times tables?

use kas::prelude::*;
use kas::view::{driver, DataAccessor, MatrixView, SelectionMode, SelectionMsg};
use kas::widgets::{column, row, EditBox, ScrollBars};
use std::ops::Range;

#[derive(Debug, Default)]
struct TableSize {
    dim: usize,
    x_len: usize,
    x_start: usize,
    y_start: usize,
    contents: Vec<usize>,
}

impl DataAccessor<(usize, usize)> for TableSize {
    type Data = usize;
    type Key = (usize, usize);
    type Item = usize;

    fn update(&mut self, dim: &Self::Data) {
        self.dim = *dim;
    }

    fn len(&self, _: &Self::Data) -> (usize, usize) {
        (self.dim, self.dim)
    }

    fn prepare_range(&mut self, _: &Self::Data, range: Range<(usize, usize)>) {
        // This is a simple hack to cache contents for the given range for usage by item()
        let x_len = range.end.0 - range.start.0;
        let y_len = range.end.1 - range.start.1;
        if x_len != self.x_len
            || x_len * y_len != self.contents.len()
            || self.x_start != range.start.0
            || self.y_start != range.start.1
        {
            self.x_len = x_len;
            self.x_start = range.start.0;
            self.y_start = range.start.1;
            self.contents.clear();
            self.contents.reserve(x_len * y_len);

            for y in range.start.1..range.end.1 {
                for x in range.start.0..range.end.0 {
                    self.contents.push((x + 1) * (y + 1));
                }
            }
        }
    }

    fn key(&self, _: &Self::Data, index: (usize, usize)) -> Option<Self::Key> {
        Some(index)
    }

    fn item(&self, _: &Self::Data, key: &Self::Key) -> Option<&Self::Item> {
        // We are required to return a reference, otherwise we would simply
        // calculate the value here!
        let (x, y) = *key;
        let xrel = x - self.x_start;
        let yrel = y - self.y_start;
        let i = xrel + yrel * self.x_len;
        self.contents.get(i)
    }
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let table = MatrixView::new(TableSize::default(), driver::NavView)
        .with_num_visible(12, 12)
        .with_selection_mode(SelectionMode::Single);
    let table = ScrollBars::new(table);

    #[derive(Debug)]
    struct SetLen(usize);

    let ui = column![
        row!["From 1 to", EditBox::parser(|dim: &usize| *dim, SetLen)],
        table.align(AlignHints::RIGHT),
    ];
    let ui = ui
        .with_state(12)
        .on_message(|_, dim, SetLen(len)| *dim = len)
        .on_message(|_, _, selection| match selection {
            SelectionMsg::<(usize, usize)>::Select((col, row)) => {
                let (c, r) = (col + 1, row + 1);
                println!("{} × {} = {}", c, r, c * r);
            }
            _ => (),
        });
    let window = Window::new(ui, "Times-Tables");

    let theme = kas::theme::SimpleTheme::new();
    kas::runner::Default::with_theme(theme)
        .build(())?
        .with(window)
        .run()
}
