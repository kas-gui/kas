// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! File system explorer

pub mod tile;
mod viewer;

use kas::prelude::*;
use kas::widgets::adapt::{MapAny, WithMarginStyle};
use kas::widgets::{Button, CheckButton, Filler, Label, Row, Slider, column, frame, row};
use kas::window::Window;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static TILE_SIZE: AtomicU32 = AtomicU32::new(128);
fn tile_size() -> u32 {
    TILE_SIZE.load(Ordering::Relaxed)
}

pub struct Data {
    pub show_hidden: bool,
    path: PathBuf,
}

type Entry = PathBuf;

#[derive(Clone, Debug)]
struct ChangeDir(PathBuf);

#[derive(Debug)]
struct ShowHidden(bool);

#[derive(Debug)]
struct TileSize(u32);

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

fn bottom_bar() -> impl Widget<Data = Data> {
    let show_hidden = CheckButton::new("Show hidden", |_, data: &Data| data.show_hidden)
        .with(|cx, _, state| cx.push(ShowHidden(state)));
    let tile_size = Slider::right(32..=512, |_, _| tile_size())
        .with_step(32)
        .with_msg(TileSize);
    row![show_hidden, tile_size]
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
        show_hidden: false,
        path,
    };

    let trail = row![
        frame!(trail()).with_style(kas::theme::FrameStyle::None),
        Filler::new().map_any()
    ];

    let ui = column![trail, viewer::viewer(), bottom_bar()]
        .with_state(data)
        .on_message(|_, state, ShowHidden(show_hidden)| {
            state.show_hidden = show_hidden;
        })
        .on_message(|cx, state, ChangeDir(path)| {
            let title = window_title(&path);
            cx.push(kas::messages::SetWindowTitle(title));
            state.path = path;
        })
        .on_message(|cx, _, TileSize(size)| {
            TILE_SIZE.store(size, Ordering::Relaxed);
            cx.resize();
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
