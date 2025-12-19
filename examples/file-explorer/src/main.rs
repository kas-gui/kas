// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! File system explorer

pub mod tile;
mod viewer;

use kas::prelude::*;
use kas::widgets::adapt::{MapAny, WithMarginStyle};
use kas::widgets::{Button, Filler, Label, Row, column, frame, row};
use kas::window::Window;
use std::io;
use std::path::{Path, PathBuf};

pub struct Data {
    path: PathBuf,
    filter_hidden: bool,
}

type Entry = PathBuf;

#[derive(Clone, Debug)]
struct ChangeDir(PathBuf);

fn trail() -> impl Widget<Data = Data> {
    Row::<Vec<MapAny<_, WithMarginStyle<Button<Label<String>>>>>>::new(vec![]).on_update(
        |cx, row, data: &Data| {
            let mut path = PathBuf::new();
            for (i, component) in data.path.iter().enumerate() {
                if path.as_os_str().is_empty() {
                    path = PathBuf::from(component);
                } else {
                    path = path.join(component);
                }

                let label = format!("{} 〉", component.display());
                if row
                    .get(i)
                    .map(|b| (***b).inner.as_str() == label)
                    .unwrap_or(false)
                {
                    continue;
                }

                row.truncate(cx, i);

                row.push(
                    cx,
                    data,
                    Button::new(Label::new(label.to_string()))
                        .with_msg(ChangeDir(path.clone()))
                        .with_frame_style(kas::theme::FrameStyle::InvisibleButton)
                        .with_margin_style(kas::theme::MarginStyle::None)
                        .map_any(),
                );
            }
        },
    )
}

fn main() -> kas::runner::Result<()> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Error)
        .filter_module("kas", log::LevelFilter::Info)
        .filter_module(module_path!(), log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let path = match std::env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            let path = PathBuf::from(".");
            report_io_error(&path, err);
            path
        }
    };
    let title = window_title(&path);

    let data = Data {
        path,
        filter_hidden: true,
    };

    let trail = row![
        frame!(trail()).with_style(kas::theme::FrameStyle::None),
        Filler::new().map_any()
    ];

    let ui = column![trail, viewer::viewer()]
        .with_state(data)
        .on_message(|cx, state, ChangeDir(path)| {
            let title = window_title(&path);
            cx.push(kas::messages::SetWindowTitle(title));
            state.path = path;
        });
    let window = Window::new(ui, title).escapable();

    kas::runner::Runner::new(())?.with(window).run()
}

fn window_title(path: &Path) -> String {
    if let Some(name) = path.file_name() {
        format!("{} — File Explorer", name.display())
    } else {
        format!("{} — File Explorer", path.display())
    }
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
