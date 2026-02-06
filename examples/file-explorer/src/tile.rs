use crate::{ChangeDir, Entry};
use image::ImageFormat;
use kas::Tile as _;
use kas::image::{Image, Svg};
use kas::prelude::*;
use kas::theme::MarginStyle;
use kas::widgets::{AdaptWidget, Button, Label, Page, Stack, Text};
use std::fmt::Write;
use std::path::{Path, PathBuf};

#[autoimpl(Debug)]
pub enum State {
    Unknown,
    Directory,
    Image(ImageFormat),
    Svg,
}

impl State {
    /// Detect from `path`
    fn detect(path: &Path) -> Self {
        if path.is_dir() {
            State::Directory
        } else if let Ok(format) = ImageFormat::from_path(path) {
            State::Image(format)
        } else if path.extension().map(|ext| ext == "svg").unwrap_or_default() {
            State::Svg
        } else {
            State::Unknown
        }
    }

    /// Return a specific stack page widget (if relevant)
    fn page(&self, path: &Path) -> Option<Page<String>> {
        match self {
            State::Unknown => None,
            Self::Directory => Some(Page::new(directory(path.to_path_buf()))),
            Self::Image(_) => Some(Page::new(image(path))),
            Self::Svg => svg(path).map(Page::new).ok(),
        }
    }
}

fn generic() -> impl Widget<Data = String> {
    Text::new_str(|text: &String| text)
}

fn directory(path: PathBuf) -> impl Widget<Data = String> {
    let name = path
        .file_name()
        .map(|os_str| os_str.to_string_lossy().to_string())
        .unwrap_or_default();
    Button::label_msg(name, ChangeDir(path)).map_any()
}

fn image(path: &Path) -> impl Widget<Data = String> + 'static {
    Image::new(path).map_any().on_update(|_, widget, _| {
        let size = crate::tile_size().cast();
        widget.set_logical_size((size, size));
    })
}

fn svg(path: &Path) -> Result<impl Widget<Data = String> + 'static, impl std::error::Error> {
    Svg::new_path(path).map(|svg| {
        svg.map_any().on_update(|_, widget, _| {
            let size = crate::tile_size().cast();
            widget.set_logical_size((size, size));
        })
    })
}

#[impl_self]
mod Tile {
    #[widget]
    #[layout(self.stack)]
    pub struct Tile {
        core: widget_core!(),
        path: PathBuf,
        generic: String,
        #[widget(&self.generic)]
        stack: Stack<String>,
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
            if *entry == self.path {
                return;
            }
            self.path.clear();
            self.path.push(entry);

            if entry.as_os_str().is_empty() {
                self.generic.replace_range(.., "loading");
            } else {
                self.generic.clear();
                write!(self.generic, "{}", entry.display()).unwrap();

                let path = self.path.clone();
                cx.send_spawn(self.id(), async move { State::detect(&path) });
            }

            // Always reset the page to 0 on change
            self.stack.set_active(cx, &self.generic, 0);
            self.stack.truncate(cx, 1);
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(state) = cx.try_pop::<State>() {
                if let Some(page) = state.page(&self.path) {
                    self.stack.push(cx, &self.generic, page);
                    self.stack.set_active(cx, &self.generic, 1);
                }
            }
        }
    }

    impl Self {
        fn new() -> Self {
            Tile {
                core: Default::default(),
                path: PathBuf::new(),
                generic: String::new(),
                stack: Stack::from([Page::new(generic())]),
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
