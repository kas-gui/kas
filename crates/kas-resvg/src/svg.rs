// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! SVG widget

use kas::draw::{ImageFormat, ImageHandle};
use kas::geom::Size;
use kas::layout::{LogicalSize, PixmapScaling};
use kas::prelude::*;
use std::io::Result;
use std::path::Path;
use tiny_skia::{Pixmap, Transform};

impl_scope! {
    /// An SVG image loaded from a path
    ///
    /// May be default constructed (result is empty).
    #[cfg_attr(doc_cfg, doc(cfg(feature = "svg")))]
    #[autoimpl(Debug ignore self.tree)]
    #[impl_default]
    #[derive(Clone)]
    #[widget]
    pub struct Svg {
        core: widget_core!(),
        tree: Option<usvg::Tree>,
        scaling: PixmapScaling,
        ideal_size: Size,
        pixmap: Option<Pixmap>,
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
            let fontdb = fonts_db.db();
            let font_family = fonts_db
                .font_family_from_alias("SERIF")
                .unwrap_or_default();

            // Defaults are taken from usvg::Options::default(). Notes:
            // - adjusting for screen scale factor is purely a property of
            //   making the canvas larger and not important here
            // - default_size: affected by screen scale factor later
            // - dpi: according to css-values-3, 1in = 96px
            // - font_size: units are (logical) px per em; 16px = 12pt
            let opts = usvg::OptionsRef {
                resources_dir,
                dpi: 96.0,
                font_family: &font_family,
                font_size: 16.0, // units: "logical pixels" per Em
                languages: &["en".to_string()],
                shape_rendering: usvg::ShapeRendering::default(),
                text_rendering: usvg::TextRendering::default(),
                image_rendering: usvg::ImageRendering::default(),
                keep_named_groups: false,
                default_size: usvg::Size::new(100.0, 100.0).unwrap(),
                fontdb,
                image_href_resolver: &Default::default(),
            };

            self.tree = Some(usvg::Tree::from_data(data, &opts).unwrap());
            self.scaling.size = self.tree.as_ref()
                .map(|tree| LogicalSize::conv(tree.svg_node().size.to_screen_size().dimensions()))
                .unwrap_or(LogicalSize(128.0, 128.0));
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
        pub fn set_scaling(&mut self, f: impl FnOnce(&mut PixmapScaling)) -> TkAction {
            f(&mut self.scaling);
            // NOTE: if only `aspect` is changed, REDRAW is enough
            TkAction::RESIZE
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.scaling.size_rules(size_mgr, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
            let scale_factor = mgr.size_mgr().scale_factor();
            self.core.rect = self.scaling.align_rect(rect, align, scale_factor);
            let size: (u32, u32) = self.core.rect.size.cast();

            let pm_size = self.pixmap.as_ref().map(|pm| (pm.width(), pm.height()));
            if pm_size.unwrap_or((0, 0)) != size {
                if let Some(handle) = self.image.take() {
                    mgr.draw_shared().image_free(handle);
                }
                self.pixmap = Pixmap::new(size.0, size.1);
                if let Some(tree) = self.tree.as_ref() {
                    self.image = self.pixmap.as_mut().map(|pm| {
                        let (w, h) = (pm.width(), pm.height());

                        let transform = Transform::identity();
                        resvg::render(tree, usvg::FitTo::Size(w, h), transform, pm.as_mut());

                        let handle = mgr.draw_shared().image_alloc((w, h)).unwrap();
                        mgr.draw_shared().image_upload(&handle, pm.data(), ImageFormat::Rgba8);
                        handle
                    });
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
