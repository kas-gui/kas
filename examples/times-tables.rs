//! Do you know your times tables?

use kas::prelude::*;
use kas::updatable::{MatrixData, Updatable};
use kas::widgets::view::{driver::DefaultNav, MatrixView, SelectionMode};
use kas::widgets::{EditBox, ScrollBars, StrLabel, Window};

#[derive(Debug)]
struct TableData(u64, usize);
impl Updatable<(usize, usize), VoidMsg> for TableData {
    fn handle(&self, _: &(usize, usize), _: &VoidMsg) -> Option<UpdateHandle> {
        None
    }
}
impl MatrixData for TableData {
    type ColKey = usize;
    type RowKey = usize;
    type Key = (usize, usize);
    type Item = usize;

    fn update_handles(&self) -> Vec<UpdateHandle> {
        vec![]
    }
    fn version(&self) -> u64 {
        self.0
    }

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

    fn contains(&self, key: &Self::Key) -> bool {
        key.0 < self.1 && key.1 < self.1
    }
    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        self.contains(key).then(|| (key.0 + 1) * (key.1 + 1))
    }

    fn update(&self, _: &Self::Key, _: Self::Item) -> Option<UpdateHandle> {
        None
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

    let layout = make_widget! {
        #[widget{
            layout = column: [
                row: [self.label, self.max],
                align(right): self.table,
            ];
            msg = VoidMsg;
        }]
        struct {
            #[widget] label = StrLabel::new("From 1 to"),
            #[widget(use_msg = set_max)] max: impl HasString = EditBox::new("12")
                .on_afl(|text, _| text.parse::<usize>().ok()),
            #[widget(discard_msg)] table: ScrollBars<MatrixView<TableData, DefaultNav>> = table,
        }
        impl Self {
            fn set_max(&mut self, mgr: &mut EventMgr, max: usize) {
                let data = self.table.data_mut();
                if data.1 != max {
                    data.0 += 1;
                    data.1 = max;
                    self.table.update_view(mgr);
                }
            }
        }
    };
    let window = Window::new("Times-Tables", layout);

    let theme = kas::theme::ShadedTheme::new().with_font_size(16.0);
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
