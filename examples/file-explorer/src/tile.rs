use crate::{ChangeDir, Entry};
use image::ImageFormat;
use kas::Tile as _;
use kas::image::{Image, Svg};
use kas::prelude::*;
use kas::text::LineIterator;
use kas::theme::{MarginStyle, TextClass};
use kas::widgets::{AdaptWidget, Button, Frame, Label, Page, Stack, Text};
use std::fmt::Write;
use std::path::{Path, PathBuf};

#[autoimpl(Debug ignore self.0)]
struct SendBoxedWidget(Box<dyn Widget<Data = String> + Send>);
impl SendBoxedWidget {
    #[inline]
    fn new(w: impl Widget<Data = String> + Send + 'static) -> Self {
        SendBoxedWidget(Box::new(w))
    }
}

/// Detect from `path`
///
/// Returns a specific stack page widget (if relevant)
fn detect(path: &Path) -> Option<SendBoxedWidget> {
    if path.is_dir() {
        Some(directory(path.to_path_buf()))
    } else if let Ok(_format) = ImageFormat::from_path(path) {
        Some(image(path))
    } else if let Some(ext) = path.extension() {
        if ext == "svg" {
            svg(path).ok()
        } else if ext == "txt" || ext == "md" {
            TextTile::new(path)
                .map(|w| SendBoxedWidget::new(Frame::new(w)))
                .ok()
        } else {
            None
        }
    } else {
        None
    }
}

fn generic() -> impl Widget<Data = String> {
    Text::new_str(|text: &String| text)
}

fn directory(path: PathBuf) -> SendBoxedWidget {
    let name = path
        .file_name()
        .map(|os_str| os_str.to_string_lossy().to_string())
        .unwrap_or_default();
    SendBoxedWidget::new(Button::label_msg(name, ChangeDir(path)).map_any())
}

fn image(path: &Path) -> SendBoxedWidget {
    SendBoxedWidget::new(Image::new(path).map_any().on_update(|_, widget, _| {
        let size = crate::tile_size().cast();
        widget.set_logical_size((size, size));
    }))
}

fn svg(path: &Path) -> Result<SendBoxedWidget, impl std::error::Error> {
    match Svg::new_path(path) {
        Ok(svg) => Ok(SendBoxedWidget::new(svg.map_any().on_update(
            |_, widget, _| {
                let size = crate::tile_size().cast();
                widget.set_logical_size((size, size));
            },
        ))),
        Err(e) => Err(e),
    }
}

#[impl_self]
mod TextTile {
    #[widget]
    #[layout(self.text)]
    pub struct TextTile {
        core: widget_core!(),
        text: kas::theme::Text<String>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let _ = self.text.size_rules(cx, axis);
            let size = crate::tile_size().cast();
            cx.logical(size, size).build(axis)
        }
    }

    impl Events for Self {
        type Data = String;
    }

    impl Self {
        fn new(path: &Path) -> Result<Self, std::io::Error> {
            // We truncate the text to an arbitrary line limit.
            // TODO(opt): limit during file reading.
            const LINES: usize = 16;
            let mut text = std::fs::read_to_string(path)?;
            let len = text.len();
            let len = LineIterator::new(&text)
                .take(LINES)
                .last()
                .map(|range| range.end)
                .unwrap_or(len);
            text.truncate(len);

            Ok(TextTile {
                core: Default::default(),
                text: kas::theme::Text::new(text, TextClass::Small, true),
            })
        }
    }
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
                cx.send_spawn(self.id(), async move { detect(&path) });
            }

            // Always reset the page to 0 on change
            self.stack.set_active(cx, &self.generic, 0);
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(Some(SendBoxedWidget(w))) = cx.try_pop() {
                self.stack.truncate(cx, 1);
                self.stack.push(cx, &self.generic, Page::new_boxed(w));
                self.stack.set_active(cx, &self.generic, 1);
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
