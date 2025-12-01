//! Main content viewers

mod dir;

use kas::prelude::*;
use std::path::PathBuf;

pub fn viewer() -> impl Widget<Data = PathBuf> {
    dir::DirView::default()
}
