// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! File system explorer

pub mod tile;
mod viewer;

use kas::prelude::*;
use kas::window::Window;
use std::io;
use std::path::{Path, PathBuf};

type Entry = Result<PathBuf, ()>;

fn main() -> kas::runner::Result<()> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Error)
        .filter_module("kas", log::LevelFilter::Info)
        .filter_module(module_path!(), log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let ui = viewer::viewer().with_state(PathBuf::from("."));
    let window = Window::new(ui, "File System Explorer").escapable();

    kas::runner::Runner::new(())?.with(window).run()
}

fn report_io_error(path: &Path, err: io::Error) {
    log::warn!("IO error: {err}");
    log::warn!("For path: \"{}\"", path.display());
    let inner = err.into_inner();
    let mut cause = inner
        .as_ref()
        .map(|err| &**err as &(dyn std::error::Error + 'static));
    while let Some(err) = cause {
        log::warn!("Cause: {err}");
        cause = err.source();
    }
}
