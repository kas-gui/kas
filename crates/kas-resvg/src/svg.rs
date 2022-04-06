// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! SVG widget

// TODO: error handling (unwrap)

use kas::draw::{ImageFormat, ImageId};
use kas::geom::Size;
use kas::layout::MarginSelector;
use kas::prelude::*;
use std::path::PathBuf;
use tiny_skia::{Pixmap, Transform};

#[derive(Copy, Clone, Debug)]
enum Scale {
    Factors(f32, f32),
    Size(LogicalSize),
}

impl_scope! {
    /// An SVG image loaded from a path
    #[cfg_attr(doc_cfg, doc(cfg(feature = "svg")))]
    #[autoimpl(Debug ignore self.tree)]
    #[derive(Clone)]
    #[widget]
    pub struct Svg {
        #[widget_core]
        core: CoreData,
        path: PathBuf,
        tree: Option<usvg::Tree>,
        margins: MarginSelector,
        scale: Scale,
        ideal_size: Size,
        stretch: Stretch,
        pixmap: Option<Pixmap>,
        image_id: Option<ImageId>,
    }

    impl Svg {
        /// Load from a path
        ///
        /// Uses the SVG's embedded size adjusted by the window's scale factor.
        pub fn load<P: Into<PathBuf>>(path: P) -> Self {
            Self::load_with_factors(path, 1.0, 1.0)
        }

        /// Load from a path and size factors
        ///
        /// Uses the SVG's embedded size, scaled by the given min or ideal
        /// factor then by the window's scale factor.
        pub fn load_with_factors<P: Into<PathBuf>>(
            path: P,
            min_size_factor: f32,
            ideal_size_factor: f32,
        ) -> Self {
            Svg {
                core: Default::default(),
                path: path.into(),
                tree: None,
                margins: MarginSelector::Outer,
                scale: Scale::Factors(min_size_factor, ideal_size_factor),
                ideal_size: Size::ZERO,
                stretch: Stretch::Low,
                pixmap: None,
                image_id: None,
            }
        }

        /// Load from a path and size
        ///
        /// Ignore's the SVG's embedded size, instead using the given size,
        /// scaled by the window's scale factor.
        pub fn load_with_size<P: Into<PathBuf>>(path: P, size: LogicalSize) -> Self {
            Svg {
                core: Default::default(),
                path: path.into(),
                tree: None,
                margins: MarginSelector::Outer,
                scale: Scale::Size(size),
                ideal_size: Size::ZERO,
                stretch: Stretch::Low,
                pixmap: None,
                image_id: None,
            }
        }

        /// Set margins
        #[must_use]
        pub fn with_margins(mut self, margins: MarginSelector) -> Self {
            self.margins = margins;
            self
        }

        /// Set stretch policy
        #[must_use]
        pub fn with_stretch(mut self, stretch: Stretch) -> Self {
            self.stretch = stretch;
            self
        }

        /// Set margins
        pub fn set_margins(&mut self, margins: MarginSelector) {
            self.margins = margins;
        }

        /// Set stretch policy
        pub fn set_stretch(&mut self, stretch: Stretch) {
            self.stretch = stretch;
        }
    }

    impl WidgetConfig for Svg {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            if self.tree.is_none() {
                // TODO: maybe we should use a singleton to deduplicate loading by
                // path? Probably not much use for duplicate SVG widgets however.
                let data = std::fs::read(&self.path).unwrap();

                // TODO: should we reload the SVG if the scale factor changes?
                let size_mgr = mgr.size_mgr();
                let scale_factor = size_mgr.scale_factor();
                let def_size = 100.0 * f64::conv(scale_factor);

                let fonts_db = kas::text::fonts::fonts().read_db();
                let fontdb = fonts_db.db();
                let font_family = fonts_db
                    .font_family_from_alias("SERIF")
                    .unwrap_or_default();
                let font_size = size_mgr.pixels_from_em(1.0) as f64;

                // TODO: some options here should be configurable
                let opts = usvg::OptionsRef {
                    resources_dir: self.path.parent(),
                    dpi: 96.0 * f64::conv(scale_factor),
                    font_family: &font_family,
                    font_size,
                    languages: &[],
                    shape_rendering: usvg::ShapeRendering::default(),
                    text_rendering: usvg::TextRendering::default(),
                    image_rendering: usvg::ImageRendering::default(),
                    keep_named_groups: false,
                    default_size: usvg::Size::new(def_size, def_size).unwrap(),
                    fontdb,
                    image_href_resolver: &Default::default(),
                };

                self.tree = Some(usvg::Tree::from_data(&data, &opts).unwrap());
            }
        }
    }

    impl Layout for Svg {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let scale_factor = size_mgr.scale_factor();

            let opt_size = self.tree.as_ref().map(|tree|
                tree.svg_node().size.to_screen_size().dimensions());
            let mut size = LogicalSize::from(opt_size.unwrap_or((128, 128)));

            let (min_factor, ideal_factor) = match self.scale {
                Scale::Factors(min, ideal) => (min * scale_factor, ideal * scale_factor),
                Scale::Size(explicit_size) => {
                    size = explicit_size;
                    (scale_factor, scale_factor)
                }
            };

            self.ideal_size = size.to_physical(ideal_factor);
            let min_size = size.extract_scaled(axis, min_factor);
            let ideal_size = self.ideal_size.extract(axis);

            let margins = self.margins.select(size_mgr).extract(axis);
            SizeRules::new(min_size, ideal_size, margins, self.stretch)
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            let size = match self.ideal_size.aspect_scale_to(rect.size) {
                Some(size) => {
                    self.core_data_mut().rect = align
                        .complete(Align::Center, Align::Center)
                        .aligned_rect(size, rect);
                    Cast::<(u32, u32)>::cast(size)
                }
                None => {
                    self.core_data_mut().rect = rect;
                    self.pixmap = None;
                    self.image_id = None;
                    return;
                }
            };

            let pm_size = self.pixmap.as_ref().map(|pm| (pm.width(), pm.height()));
            if pm_size.unwrap_or((0, 0)) != size {
                if let Some(id) = self.image_id {
                    mgr.draw_shared().image_free(id);
                }
                self.pixmap = Pixmap::new(size.0, size.1);
                if let Some(tree) = self.tree.as_ref() {
                    self.image_id = self.pixmap.as_mut().map(|pm| {
                        let (w, h) = (pm.width(), pm.height());

                        // alas, we cannot tell resvg to skip the aspect-ratio-scaling!
                        let transform = Transform::identity();
                        resvg::render(tree, usvg::FitTo::Height(h), transform, pm.as_mut());

                        let id = mgr.draw_shared().image_alloc((w, h)).unwrap();
                        mgr.draw_shared().image_upload(id, pm.data(), ImageFormat::Rgba8);
                        id
                    });
                }
            }
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            if let Some(id) = self.image_id {
                draw.image(self, id);
            }
        }
    }
}
