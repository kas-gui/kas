//! Do you know your times tables?

use kas::prelude::*;
use kas::updatable::{MatrixData, Updatable, UpdatableHandler};
use kas::widgets::{view::MatrixView, EditBox, StrLabel, Window};

#[derive(Debug)]
struct TableData(u64, usize);
impl Updatable for TableData {
    fn update_handle(&self) -> Option<UpdateHandle> {
        None
    }
}
impl UpdatableHandler<(usize, usize), VoidMsg> for TableData {
    fn handle(&self, _: &(usize, usize), _: &VoidMsg) -> Option<UpdateHandle> {
        None
    }
}
impl MatrixData for TableData {
    type ColKey = usize;
    type RowKey = usize;
    type Key = (usize, usize);
    type Item = usize;

    fn version(&self) -> u64 {
        self.0
    }

    fn col_len(&self) -> usize {
        self.1
    }
    fn row_len(&self) -> usize {
        self.1
    }

    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
        parent.make_child(key.0).make_child(key.1)
    }
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
        let mut iter = child.iter_keys_after(parent);
        let row = iter.next();
        let col = iter.next();
        row.zip(col)
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
        (start..limit).collect()
    }
    fn row_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::RowKey> {
        (start..limit).collect()
    }

    fn make_key(row: &Self::RowKey, col: &Self::ColKey) -> Self::Key {
        (*row, *col)
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let layout = make_widget! {
        #[widget{
            layout = column: [
                row: [self.label, self.max],
                align(right): self.table,
            ];
        }]
        #[handler(msg = VoidMsg)]
        struct {
            #[widget] label = StrLabel::new("From 1 to"),
            #[widget(use_msg = set_max)] max: impl HasString = EditBox::new("12")
                .on_afl(|text, _| text.parse::<usize>().ok()),
            #[widget(discard_msg)] table: MatrixView<TableData> =
                MatrixView::new(TableData(0, 12)),
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
