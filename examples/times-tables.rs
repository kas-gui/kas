//! Do you know your times tables?

use kas::model::{MatrixData, SharedData};
use kas::prelude::*;
use kas::view::{driver, MatrixView, SelectionMode};
use kas::widgets::{EditBox, ScrollBars};

#[derive(Debug)]
struct TableData(u64, usize);
impl SharedData for TableData {
    type Key = (usize, usize);
    type Item = usize;

    fn version(&self) -> u64 {
        self.0
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        key.0 < self.1 && key.1 < self.1
    }
    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        self.contains_key(key).then(|| (key.0 + 1) * (key.1 + 1))
    }

    fn update(&self, _: &mut EventMgr, _: &Self::Key, _: Self::Item) {}
}
impl MatrixData for TableData {
    type ColKey = usize;
    type RowKey = usize;

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

    fn col_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::ColKey> {
        (start..(start + limit)).collect()
    }
    fn row_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::RowKey> {
        (start..(start + limit)).collect()
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

    let window = impl_singleton! {
        #[widget{
            layout = column: [
                row: ["From 1 to", self.max],
                align(right): self.table,
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] max: impl HasString = EditBox::new("12")
                .on_afl(|text, mgr| match text.parse::<usize>() {
                    Ok(n) => mgr.push_msg(n),
                    Err(_) => (),
                }),
            #[widget] table: ScrollBars<MatrixView<TableData, driver::NavView>> = table,
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, index: usize) {
                if index == widget_index![self.max] {
                    if let Some(max) = mgr.try_pop_msg::<usize>() {
                        let data = self.table.data_mut();
                        if data.1 != max {
                            data.0 += 1;
                            data.1 = max;
                            self.table.update_view(mgr);
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
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
