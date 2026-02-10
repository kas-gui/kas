//! Main content viewers

mod dir;

use kas::prelude::*;

pub fn viewer() -> impl Widget<Data = crate::Data> {
    dir::DirView::new()
}
