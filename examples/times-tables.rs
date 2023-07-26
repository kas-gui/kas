//! Do you know your times tables?

use kas::prelude::*;
use kas::view::{driver, MatrixData, MatrixView, SelectionMode, SharedData};
use kas::widget::{Adapt, EditBox, ScrollBars};

#[derive(Debug)]
struct TableSize(usize);
impl SharedData for TableSize {
    type Key = (usize, usize);
    type Item = usize;
    type ItemRef<'b> = usize;

    fn contains_key(&self, key: &Self::Key) -> bool {
        key.0 < self.0 && key.1 < self.0
    }
    fn borrow(&self, key: &Self::Key) -> Option<Self::ItemRef<'_>> {
        self.contains_key(key).then_some((key.0 + 1) * (key.1 + 1))
    }
}
impl MatrixData for TableSize {
    type ColKey = usize;
    type RowKey = usize;

    type ColKeyIter<'b> = std::ops::Range<usize>;
    type RowKeyIter<'b> = std::ops::Range<usize>;

    fn is_empty(&self) -> bool {
        self.0 == 0
    }
    fn len(&self) -> (usize, usize) {
        (self.0, self.0)
    }

    fn col_iter_from(&self, start: usize, limit: usize) -> std::ops::Range<usize> {
        let end = self.0.min(start + limit);
        start..end
    }
    fn row_iter_from(&self, start: usize, limit: usize) -> std::ops::Range<usize> {
        let end = self.0.min(start + limit);
        start..end
    }

    fn make_key(&self, col: &Self::ColKey, row: &Self::RowKey) -> Self::Key {
        (*col, *row)
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let table = MatrixView::new(driver::NavView)
        .with_num_visible(12, 12)
        .with_selection_mode(SelectionMode::Single);
    let table = ScrollBars::new(table);

    #[derive(Debug)]
    struct SetLen(usize);

    let ui = kas::column![
        row![
            "From 1 to",
            EditBox::parser(|data: &TableSize| data.0, SetLen)
        ],
        align!(right, table),
    ];
    let ui = Adapt::new(ui, TableSize(12)).on_message(|_, data, SetLen(len)| data.0 = len);
    let window = Window::new(ui, "Times-Tables");

    let theme = kas::theme::SimpleTheme::new().with_font_size(16.0);
    kas::shell::DefaultShell::new((), theme)?
        .with(window)?
        .run()
}
