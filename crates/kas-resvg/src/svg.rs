// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! SVG widget

use kas::draw::{ImageFormat, ImageHandle};
use kas::layout::{LogicalSize, PixmapScaling};
use kas::prelude::*;
use std::io::Result;
use std::path::Path;
use tiny_skia::{Pixmap, Transform};
use usvg::Tree;

#[derive(Clone, Default)]
enum State {
    #[default]
    None,
    Initial(Tree),
    Ready(Tree, Pixmap),
}

fn draw(tree: Tree, mut pixmap: Pixmap) -> (Tree, Pixmap) {
    let (w, h) = (pixmap.width(), pixmap.height());
    let transform = Transform::identity();
    resvg::render(&tree, usvg::FitTo::Size(w, h), transform, pixmap.as_mut());
    (tree, pixmap)
}

impl State {
    /// Resize if required, redrawing on resize
    ///
    /// Returns a future to redraw. Does nothing if currently redrawing.
    fn resize(&mut self, (w, h): (u32, u32)) -> Option<&[u8]> {
        let old_state = std::mem::replace(self, State::None);
        let (tree, pixmap) = match old_state {
            State::Ready(tree, px) if (px.width(), px.height()) == (w, h) => {
                *self = State::Ready(tree, px);
                return None;
            }
            State::None => return None,
            State::Initial(tree) | State::Ready(tree, _) => {
                if let Some(px) = Pixmap::new(w, h) {
                    (tree, px)
                } else {
                    *self = State::Initial(tree);
                    return None;
                }
            }
        };

        let (tree, px) = draw(tree, pixmap);
        *self = State::Ready(tree, px);
        if let State::Ready(_, ref px) = self {
            Some(px.data())
        } else {
            None // unreachable
        }
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
        pub fn new(data: &[u8]) -> Self {
            let mut svg = Svg::default();
            let _ = svg.load(data, None);
            svg
        }

        /// Construct from a path
        pub fn new_path<P: AsRef<Path>>(path: P) -> Result<Self> {
            let mut svg = Svg::default();
            let _ = svg.load_path(path)?;
            Ok(svg)
        }

        /// Load from `data`
        ///
        /// Replaces existing data, but does not re-render until a resize
        /// happens (hence returning [`Action::REZISE`]).
        ///
        /// This sets [`PixmapScaling::size`] from the SVG.
        pub fn load(&mut self, data: &[u8], resources_dir: Option<&Path>) -> Action {
            let fonts_db = kas::text::fonts::fonts().read_db();
            let font_family = fonts_db.font_family_from_alias("SERIF").unwrap_or_default();

            // Defaults are taken from usvg::Options::default(). Notes:
            // - adjusting for screen scale factor is purely a property of
            //   making the canvas larger and not important here
            // - default_size: affected by screen scale factor later
            // - dpi: according to css-values-3, 1in = 96px
            // - font_size: units are (logical) px per em; 16px = 12pt
            let opts = usvg::Options {
                resources_dir: resources_dir.map(|path| path.to_owned()),
                dpi: 96.0,
                font_family,
                font_size: 16.0, // units: "logical pixels" per Em
                languages: vec!["en".to_string()],
                shape_rendering: usvg::ShapeRendering::default(),
                text_rendering: usvg::TextRendering::default(),
                image_rendering: usvg::ImageRendering::default(),
                keep_named_groups: false,
                default_size: usvg::Size::new(100.0, 100.0).unwrap(),
                image_href_resolver: Default::default(),
            };

            let tree = usvg::Tree::from_data(data, &opts).unwrap();
            self.scaling.size = LogicalSize::conv(tree.size.to_screen_size().dimensions());
            self.inner = match std::mem::take(&mut self.inner) {
                State::Ready(_, px) => State::Ready(tree, px),
                _ => State::Initial(tree),
            };
            Action::RESIZE
        }

        /// Load from a path
        ///
        /// This is a wrapper around [`Self::load`].
        pub fn load_path<P: AsRef<Path>>(&mut self, path: P) -> Result<Action> {
            let data = std::fs::read(path.as_ref())?;
            let resources_dir = path.as_ref().parent();
            Ok(self.load(&data, resources_dir))
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

            if let Some(data) = self.inner.resize(size) {
                let ds = mgr.draw_shared();
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
                    ds.image_upload(handle, data, ImageFormat::Rgba8);
                }
            }
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            if let Some(id) = self.image.as_ref().map(|h| h.id()) {
                draw.image(self.rect(), id);
            }
        }
    }
}
