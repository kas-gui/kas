// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! SVG widget

use kas::draw::{ImageFormat, ImageHandle};
use kas::layout::{LogicalSize, PixmapScaling};
use kas::prelude::*;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tiny_skia::{Pixmap, Transform};
use usvg::Tree;

/// Load errors
#[derive(thiserror::Error, Debug)]
enum LoadError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("SVG error")]
    Svg(#[from] usvg::Error),
}

fn load(data: &[u8], resources_dir: Option<&Path>) -> Result<Tree, usvg::Error> {
    use once_cell::sync::Lazy;
    static FONT_FAMILY: Lazy<String> = Lazy::new(|| {
        let fonts_db = kas::text::fonts::fonts().read_db();
        fonts_db.font_family_from_alias("SERIF").unwrap_or_default()
    });

    // Defaults are taken from usvg::Options::default(). Notes:
    // - adjusting for screen scale factor is purely a property of
    //   making the canvas larger and not important here
    // - default_size: affected by screen scale factor later
    // - dpi: according to css-values-3, 1in = 96px
    // - font_size: units are (logical) px per em; 16px = 12pt
    let opts = usvg::Options {
        resources_dir: resources_dir.map(|path| path.to_owned()),
        dpi: 96.0,
        font_family: FONT_FAMILY.clone(),
        font_size: 16.0, // units: "logical pixels" per Em
        languages: vec!["en".to_string()],
        shape_rendering: usvg::ShapeRendering::default(),
        text_rendering: usvg::TextRendering::default(),
        image_rendering: usvg::ImageRendering::default(),
        default_size: usvg::Size::new(100.0, 100.0).unwrap(),
        image_href_resolver: Default::default(),
    };

    usvg::Tree::from_data(data, &opts)
}

#[derive(Clone)]
enum Source {
    Static(&'static [u8], Option<PathBuf>),
    Heap(Arc<[u8]>, Option<PathBuf>),
}
impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Static(_, path) => write!(f, "Source::Static(_, {path:?}"),
            Source::Heap(_, path) => write!(f, "Source::Heap(_, {path:?}"),
        }
    }
}
impl Source {
    fn tree(&self) -> Result<Tree, usvg::Error> {
        let (data, res_dir) = match self {
            Source::Static(d, p) => (*d, p.as_ref()),
            Source::Heap(d, p) => (&**d, p.as_ref()),
        };
        load(data, res_dir.map(|p| p.as_ref()))
    }
}

#[derive(Clone, Default)]
enum State {
    #[default]
    None,
    Initial(Source),
    Rendering(Source),
    Ready(Source, Pixmap),
}

async fn draw(svg: Source, mut pixmap: Pixmap) -> Pixmap {
    let (w, h) = (pixmap.width(), pixmap.height());
    if let Ok(tree) = svg.tree() {
        let transform = Transform::identity();
        resvg::render(&tree, usvg::FitTo::Size(w, h), transform, pixmap.as_mut());
    }
    pixmap
}

impl State {
    /// Resize if required, redrawing on resize
    ///
    /// Returns a future to redraw. Does nothing if currently redrawing.
    fn resize(&mut self, (w, h): (u32, u32)) -> Option<impl Future<Output = Pixmap>> {
        let old_state = std::mem::replace(self, State::None);
        match old_state {
            State::None => (),
            state @ State::Rendering(_) => *self = state,
            State::Ready(svg, px) if (px.width(), px.height()) == (w, h) => {
                *self = State::Ready(svg, px);
                return None;
            }
            State::Initial(svg) | State::Ready(svg, _) => {
                if let Some(px) = Pixmap::new(w, h) {
                    *self = State::Rendering(svg.clone());
                    return Some(draw(svg, px));
                } else {
                    *self = State::Initial(svg);
                    return None;
                }
            }
        }
        None
    }
}

impl_scope! {
    /// An SVG image loaded from a path
    ///
    /// May be default constructed (result is empty).
    #[cfg_attr(doc_cfg, doc(cfg(feature = "svg")))]
    #[autoimpl(Debug ignore self.inner)]
    #[impl_default]
    #[derive(Clone)]
    #[widget]
    pub struct Svg {
        core: widget_core!(),
        inner: State,
        scaling: PixmapScaling,
        image: Option<ImageHandle>,
    }

    impl Self {
        /// Construct from data
        ///
        /// Returns an error if the SVG fails to parse. If using this method
        /// with [`include_bytes`] it is probably safe to unwrap.
        pub fn new(data: &'static [u8]) -> Result<Self, impl std::error::Error> {
            let mut svg = Svg::default();
            let source = Source::Static(data, None);
            svg.load_source(source).map(|_action| svg)
        }

        /// Construct from a path
        pub fn new_path<P: AsRef<Path>>(path: P) -> Result<Self, impl std::error::Error> {
            let mut svg = Svg::default();
            let _action = svg.load_path_(path.as_ref())?;
            Result::<Self, LoadError>::Ok(svg)
        }

        /// Load from `data`
        ///
        /// Replaces existing data, but does not re-render until a resize
        /// happens (hence returning [`Action::RESIZE`]).
        ///
        /// This sets [`PixmapScaling::size`] from the SVG.
        pub fn load(&mut self, data: &'static [u8], resources_dir: Option<&Path>)
            -> Result<Action, impl std::error::Error>
        {
            let source = Source::Static(data, resources_dir.map(|p| p.to_owned()));
            self.load_source(source)
        }

        fn load_source(&mut self, source: Source) -> Result<Action, usvg::Error> {
            // Set scaling size. TODO: this is useless if Self::with_size is called after.
            let tree = source.tree()?;
            self.scaling.size = LogicalSize::conv(tree.size.to_screen_size().dimensions());

            self.inner = match std::mem::take(&mut self.inner) {
                State::Ready(_, px) => State::Ready(source, px),
                _ => State::Initial(source),
            };
            Ok(Action::RESIZE)
        }

        /// Load from a path
        ///
        /// This is a wrapper around [`Self::load`].
        pub fn load_path<P: AsRef<Path>>(&mut self, path: P)
            -> Result<Action, impl std::error::Error>
        {
            self.load_path_(path.as_ref())
        }

        fn load_path_(&mut self, path: &Path) -> Result<Action, LoadError> {
            let buf = std::fs::read(path)?;
            let rd = path.parent().map(|path| path.to_owned());
            let source = Source::Heap(buf.into(), rd);
            Ok(self.load_source(source)?)
        }

        /// Assign size
        ///
        /// By default, size is derived from the loaded SVG. See also
        /// [`Self::with_scaling`] and [`Self::set_scaling`] for more options.
        #[inline]
        #[must_use]
        pub fn with_size(mut self, size: LogicalSize) -> Self {
            self.scaling.size = size;
            self
        }

        /// Adjust scaling
        ///
        /// [`PixmapScaling::size`] is set from the SVG on loading (it may also be set here).
        /// Other scaling parameters take their default values from [`PixmapScaling`].
        #[inline]
        #[must_use]
        pub fn with_scaling(mut self, f: impl FnOnce(&mut PixmapScaling)) -> Self {
            f(&mut self.scaling);
            self
        }

        /// Adjust scaling
        ///
        /// [`PixmapScaling::size`] is set from the SVG on loading (it may also be set here).
        /// Other scaling parameters take their default values from [`PixmapScaling`].
        #[inline]
        pub fn set_scaling(&mut self, f: impl FnOnce(&mut PixmapScaling)) -> Action {
            f(&mut self.scaling);
            // NOTE: if only `aspect` is changed, REDRAW is enough
            Action::RESIZE
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.scaling.size_rules(size_mgr, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            let scale_factor = mgr.size_mgr().scale_factor();
            self.core.rect = self.scaling.align_rect(rect, scale_factor);
            let size: (u32, u32) = self.core.rect.size.cast();

            if let Some(fut) = self.inner.resize(size) {
                mgr.ev_state().push_spawn(self.id(), fut);
            }
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            if let Some(id) = self.image.as_ref().map(|h| h.id()) {
                draw.image(self.rect(), id);
            }
        }
    }

    impl Widget for Self {
        fn handle_message(&mut self, mgr: &mut EventMgr) {
            if let Some(pixmap) = mgr.try_pop::<Pixmap>() {
                let size = (pixmap.width(), pixmap.height());
                mgr.draw_shared(|ds| {
                    if let Some(im_size) = self.image.as_ref().and_then(|h| ds.image_size(h)) {
                        if im_size != Size::conv(size) {
                            if let Some(handle) = self.image.take() {
                                ds.image_free(handle);
                            }
                        }
                    }

                    if self.image.is_none() {
                        self.image = ds.image_alloc(size).ok();
                    }

                    if let Some(handle) = self.image.as_ref() {
                        ds.image_upload(handle, pixmap.data(), ImageFormat::Rgba8);
                    }
                });

                mgr.redraw(self.id());
                let inner = std::mem::replace(&mut self.inner, State::None);
                self.inner = match inner {
                    State::None => State::None,
                    State::Initial(source) |
                    State::Rendering(source) |
                    State::Ready(source, _) => State::Ready(source, pixmap),
                };
            }
        }
    }
}
