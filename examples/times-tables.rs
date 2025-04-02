//! Do you know your times tables?

use kas::prelude::*;
use kas::view::{driver, MatrixData, MatrixView, SelectionMode, SelectionMsg, SharedData};
use kas::widgets::{column, row, EditBox, ScrollBars};

#[derive(Debug)]
struct TableSize(usize);
impl SharedData for TableSize {
    type Key = (usize, usize);
    type Item = usize;

    fn get(&self, key: &Self::Key) -> Option<usize> {
        (key.0 < self.0 && key.1 < self.0).then_some((key.0 + 1) * (key.1 + 1))
    }
}
impl MatrixData for TableSize {
    type ColKey = usize;
    type RowKey = usize;

    fn is_empty(&self) -> bool {
        self.0 == 0
    }
    fn len(&self) -> (usize, usize) {
        (self.0, self.0)
    }

    #[allow(refining_impl_trait)]
    fn col_iter_from(&self, start: usize, limit: usize) -> std::ops::Range<usize> {
        let end = self.0.min(start + limit);
        start..end
    }
    #[allow(refining_impl_trait)]
    fn row_iter_from(&self, start: usize, limit: usize) -> std::ops::Range<usize> {
        let end = self.0.min(start + limit);
        start..end
    }

    fn make_key(&self, col: &Self::ColKey, row: &Self::RowKey) -> Self::Key {
        (*col, *row)
    }
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let table = MatrixView::new(driver::NavView)
        .with_num_visible(12, 12)
        .with_selection_mode(SelectionMode::Single);
    let table = ScrollBars::new(table);

    #[derive(Debug)]
    struct SetLen(usize);

    let ui = column![
        row![
            "From 1 to",
            EditBox::parser(|data: &TableSize| data.0, SetLen)
        ],
        table.align(AlignHints::RIGHT),
    ];
    let ui = ui
        .with_state(TableSize(12))
        .on_message(|_, data, SetLen(len)| data.0 = len)
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
