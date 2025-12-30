use crate::{ChangeDir, Entry};
use image::ImageFormat;
use kas::Tile as _;
use kas::prelude::*;
use kas::theme::MarginStyle;
use kas::widgets::{AdaptWidget, Button, Label, Page, Stack, Text};
use std::borrow::Cow;
use std::path::PathBuf;

#[autoimpl(Debug)]
pub enum State {
    Initial,
    Error,
    Unknown(PathBuf),
    Directory(PathBuf, String),
    Image(PathBuf, ImageFormat),
}

impl State {
    fn path(&self) -> Option<&PathBuf> {
        Some(match self {
            State::Initial | State::Error => return None,
            State::Unknown(path) => path,
            State::Directory(path, _) => path,
            State::Image(path, _) => path,
        })
    }

    /// Update, returning `true` on change (or error)
    fn update(&mut self, entry: &Entry) -> bool {
        log::trace!("State::update: {entry:?}");

        if entry.as_os_str().is_empty() {
            if matches!(self, State::Initial) {
                return false;
            }
            *self = State::Initial;
            true
        } else if self.path() == Some(entry) {
            false
        } else {
            *self = State::Unknown(entry.clone());
            true
        }
    }

    /// Detect from `path`
    fn detect(path: PathBuf) -> Self {
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|os_str| os_str.to_string_lossy().to_string())
                .unwrap_or_default();
            State::Directory(path, name)
        } else if let Ok(format) = ImageFormat::from_path(&path) {
            State::Image(path, format)
        } else {
            State::Unknown(path)
        }
    }

    /// Get page number for Tile::stack widget
    fn page(&self) -> usize {
        match self {
            Self::Directory(_, _) => 1,
            Self::Image(_, _) => 2,
            _ => 0,
        }
    }
}

fn generic() -> impl Widget<Data = State> {
    Text::new_update(|_, entry: &State, text: &mut String| {
        let new_text: Cow<str> = match &entry {
            State::Initial => Cow::from("loading"),
            State::Error => "<error>".into(),
            State::Unknown(path) => format!("{}", path.display()).into(),
            _ => "<bad state>".into(),
        };
        if *text != new_text {
            *text = new_text.into_owned();
            true
        } else {
            false
        }
    })
}

fn directory() -> impl Widget<Data = State> {
    Button::new(Text::new_str(|state: &State| match state {
        State::Directory(_, name) => name,
        _ => "<bad state>",
    }))
    .with(|cx, state: &State| {
        if let State::Directory(path, _) = state {
            cx.push(ChangeDir(path.clone()))
        }
    })
}

fn image() -> impl Widget<Data = State> {
    use kas::image::Image;

    Image::default()
        .map_any()
        .on_update(|cx, widget, state: &State| {
            let size = crate::tile_size().cast();
            widget.set_logical_size((size, size));
            if let State::Image(path, _format) = state {
                // TODO: use format parameter?
                widget.set(cx, path);
            }
        })
}

#[impl_self]
mod Tile {
    #[widget]
    #[layout(self.stack)]
    pub struct Tile {
        core: widget_core!(),
        state: State,
        #[widget(&self.state)]
        stack: Stack<State>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let size = crate::tile_size().cast();
            let rules = cx.logical(size, size).build(axis);
            self.stack.size_rules(cx, axis).max(rules)
        }
    }

    impl Events for Self {
        type Data = Entry;

        fn update(&mut self, cx: &mut ConfigCx, entry: &Entry) {
            if self.state.update(entry) {
                // Always reset the page to 0 on change
                self.stack.set_active(cx, &self.state, 0);

                if let Some(path) = self.state.path() {
                    let path = path.clone();
                    cx.send_spawn(self.id(), async { State::detect(path) });
                }
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(state) = cx.try_pop() {
                self.state = state;
                let page = self.state.page();
                self.stack.set_active(cx, &self.state, page);
            }
        }
    }

    impl Self {
        fn new() -> Self {
            Tile {
                core: Default::default(),
                state: State::Initial,
                stack: Stack::from([
                    Page::new(generic()),
                    Page::new(directory()),
                    Page::new(image()),
                ]),
            }
        }
    }
}

#[impl_self]
mod DirItem {
    #[widget]
    #[layout(column![
        self.tile,
        self.label.align(AlignHints::CENTER),
    ].with_margin_style(MarginStyle::Huge)
    )]
    pub struct DirItem {
        core: widget_core!(),
        #[widget]
        tile: Tile,
        #[widget = &()]
        label: Label<String>,
    }

    impl Events for Self {
        type Data = Entry;

        fn update(&mut self, cx: &mut ConfigCx, entry: &Entry) {
            // TODO(opt): change detection

            let mut file_name = String::new();
            if let Some(name) = entry.file_name() {
                file_name = name.to_string_lossy().into();
            }
            self.label.set_string(cx, file_name);
        }
    }

    impl Self {
        fn new() -> Self {
            DirItem {
                core: Default::default(),
                tile: Tile::new(),
                label: Label::new(String::new()),
            }
        }
    }
}

#[derive(Default)]
pub struct Driver;

impl kas::view::Driver<usize, Entry> for Driver {
    const TAB_NAVIGABLE: bool = false;
    type Widget = DirItem;

    fn make(&mut self, _: &usize) -> DirItem {
        DirItem::new()
    }

    fn navigable(_: &DirItem) -> bool {
        false
    }
}
