//! Do you know your times tables?

use kas::model::{MatrixData, SharedData};
use kas::prelude::*;
use kas::view::{driver, MatrixView, SelectionMode};
use kas::widget::{EditBox, ScrollBars};

#[derive(Debug)]
struct TableData(u64, usize);
impl SharedData for TableData {
    type Key = (usize, usize);
    type Item = usize;
    type ItemRef<'b> = usize;

    fn version(&self) -> u64 {
        self.0
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        key.0 < self.1 && key.1 < self.1
    }
    fn borrow(&self, key: &Self::Key) -> Option<Self::ItemRef<'_>> {
        self.contains_key(key).then_some((key.0 + 1) * (key.1 + 1))
    }
}
impl MatrixData for TableData {
    type ColKey = usize;
    type RowKey = usize;

    type ColKeyIter<'b> = std::ops::Range<usize>;
    type RowKeyIter<'b> = std::ops::Range<usize>;

    fn is_empty(&self) -> bool {
        self.1 == 0
    }
    fn len(&self) -> (usize, usize) {
        (self.1, self.1)
    }

    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
        parent.make_child(key.0).make_child(key.1)
    }
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
        let mut iter = child.iter_keys_after(parent);
        let col = iter.next();
        let row = iter.next();
        col.zip(row)
    }

    fn col_iter_from(&self, start: usize, limit: usize) -> std::ops::Range<usize> {
        start..(start + limit)
    }
    fn row_iter_from(&self, start: usize, limit: usize) -> std::ops::Range<usize> {
        start..(start + limit)
    }

    fn make_key(col: &Self::ColKey, row: &Self::RowKey) -> Self::Key {
        (*col, *row)
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let table = MatrixView::new(TableData(1, 12))
        .with_num_visible(12, 12)
        .with_selection_mode(SelectionMode::Single);
    let table = ScrollBars::new(table);

    let window = singleton! {
        #[widget{
            layout = column: [
                row: ["From 1 to", self.max],
                align(right): self.table,
            ];
        }]
        struct {
            core: widget_core!(),
            #[widget] max: impl Widget + HasString = EditBox::new("12")
                .on_afl(|mgr, text| match text.parse::<usize>() {
                    Ok(n) => mgr.push(n),
                    Err(_) => (),
                }),
            #[widget] table: ScrollBars<MatrixView<TableData, driver::NavView>> = table,
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr) {
                if mgr.last_child() == Some(widget_index![self.max]) {
                    if let Some(max) = mgr.try_pop::<usize>() {
                        let data = self.table.data_mut();
                        if data.1 != max {
                            data.0 += 1;
                            data.1 = max;
                            mgr.update_all(0);
                        }
                    }
                }
            }
        }
        impl Window for Self {
            fn title(&self) -> &str { "Times-Tables" }
        }
    };

    let theme = kas::theme::SimpleTheme::new().with_font_size(16.0);
    kas::shell::DefaultShell::new(theme)?.with(window)?.run()
}
