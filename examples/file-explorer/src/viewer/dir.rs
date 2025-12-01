//! Main view of a directory

use kas::prelude::*;
use std::path::PathBuf;

#[impl_self]
mod DirView {
    #[widget]
    #[layout("DIR VIEW")]
    #[derive(Default)]
    pub struct DirView {
        core: widget_core!(),
    }

    impl Events for Self {
        type Data = PathBuf;
    }
}
