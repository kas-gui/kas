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

#[derive(Clone, Default)]
struct Inner {
    tree: Option<usvg::Tree>,
    pixmap: Option<Pixmap>,
}

impl Inner {
    /// Get current pixmap size
    fn size(&self) -> (u32, u32) {
        if let Some(ref pm) = self.pixmap {
            (pm.width(), pm.height())
        } else {
            (0, 0)
        }
    }

    /// Resize and render
    fn resize(&mut self, w: u32, h: u32) -> Option<((u32, u32), &[u8])> {
        self.pixmap = Pixmap::new(w, h);
        if let Some((tree, pm)) = self.tree.as_ref().zip(self.pixmap.as_mut()) {
            let transform = Transform::identity();
            resvg::render(tree, usvg::FitTo::Size(w, h), transform, pm.as_mut());
            Some(((w, h), pm.data()))
        } else {
            None
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
        inner: Inner,
        scaling: PixmapScaling,
        image: Option<ImageHandle>,
    }

    impl Self {
        /// Construct from data
        pub fn new(data: &[u8]) -> Self {
            let mut svg = Svg::default();
            svg.load(data, None);
            svg
        }

        /// Construct from a path
        pub fn new_path<P: AsRef<Path>>(path: P) -> Result<Self> {
            let mut svg = Svg::default();
            svg.load_path(path)?;
            Ok(svg)
        }

        /// Load from `data`
        ///
        /// This sets [`PixmapScaling::size`] from the SVG.
        pub fn load(&mut self, data: &[u8], resources_dir: Option<&Path>) {
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
            self.inner.tree = Some(tree);
        }

        /// Load from a path
        pub fn load_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
            let data = std::fs::read(path.as_ref())?;
            let resources_dir = path.as_ref().parent();
            self.load(&data, resources_dir);
            Ok(())
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

            if self.inner.size() != size {
                if let Some(handle) = self.image.take() {
                    mgr.draw_shared().image_free(handle);
                }
                if let Some((size, data)) = self.inner.resize(size.0, size.1) {
                    let handle = mgr.draw_shared().image_alloc(size).unwrap();
                    mgr.draw_shared()
                        .image_upload(&handle, data, ImageFormat::Rgba8);
                    self.image = Some(handle);
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
