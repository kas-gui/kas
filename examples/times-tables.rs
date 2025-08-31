//! Do you know your times tables?

use kas::prelude::*;
use kas::view::{DataClerk, GridIndex, GridView, SelectionMode, SelectionMsg, driver};
use kas::widgets::{EditBox, ScrollBars, column, row};
use kas_view::{DataChanges, DataLen, Token};

/// A cache of the visible part of our table
#[derive(Debug, Default)]
struct TableCache {
    dim: u32,
}

fn product(index: GridIndex) -> u64 {
    let x = u64::conv(index.col + 1);
    let y = u64::conv(index.row + 1);
    x * y
}

impl DataClerk<GridIndex> for TableCache {
    /// Our table is square; it's size is input.
    type Data = u32;

    /// We re-usize the index as our key.
    type Key = GridIndex;

    type Token = Token<GridIndex, u64>;

    /// Data items are `u64` since e.g. 65536² is not representable by `u32`.
    type Item = u64;

    fn update(&mut self, _: &mut ConfigCx, _: Id, dim: &Self::Data) -> DataChanges {
        if self.dim == *dim {
            DataChanges::None
        } else {
            self.dim = *dim;
            DataChanges::NoPrepared
        }
    }

    fn len(&self, _: &Self::Data, _: GridIndex) -> DataLen<GridIndex> {
        DataLen::Known(GridIndex::splat(self.dim))
    }

    fn token(&self, _: &Self::Data, index: GridIndex) -> Option<Self::Token> {
        Some(Token::new(index, product(index)))
    }

    fn item<'r>(&'r self, _: &'r Self::Data, token: &'r Self::Token) -> &'r Self::Item {
        &token.item
    }
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let table = GridView::new(TableCache::default(), driver::View)
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
            SelectionMsg::<GridIndex>::Select(index) => {
                println!("{} × {} = {}", index.col + 1, index.row + 1, product(index));
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
