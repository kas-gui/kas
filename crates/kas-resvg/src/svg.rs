// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! SVG widget

// TODO: error handling (unwrap)

use kas::draw::{ImageFormat, ImageId};
use kas::geom::Size;
use kas::layout::{MarginSelector, SpriteDisplay, SpriteSize};
use kas::prelude::*;
use std::path::PathBuf;
use tiny_skia::{Pixmap, Transform};

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
        sprite: SpriteDisplay,
        ideal_size: Size,
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
                sprite: SpriteDisplay {
                    margins: MarginSelector::Outer,
                    size: SpriteSize::Relative(min_size_factor),
                    ideal_factor: ideal_size_factor / min_size_factor,
                    stretch: Stretch::Low,
                    ..Default::default()
                },
                ideal_size: Size::ZERO,
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
                sprite: SpriteDisplay {
                    margins: MarginSelector::Outer,
                    size: SpriteSize::Logical(size),
                    ideal_factor: 1.0,
                    stretch: Stretch::Low,
                    ..Default::default()
                },
                ideal_size: Size::ZERO,
                pixmap: None,
                image_id: None,
            }
        }

        /// Adjust scaling
        #[inline]
        #[must_use]
        pub fn with_scaling(mut self, f: impl FnOnce(SpriteDisplay) -> SpriteDisplay) -> Self {
            self.sprite = f(self.sprite);
            self
        }

        /// Adjust scaling
        #[inline]
        pub fn set_scaling(&mut self, f: impl FnOnce(&mut SpriteDisplay)) -> TkAction {
            f(&mut self.sprite);
            // NOTE: if only `aspect` is changed, REDRAW is enough
            TkAction::RESIZE
        }
    }

    impl WidgetConfig for Svg {
        fn configure(&mut self, _: &mut SetRectMgr) {
            if self.tree.is_none() {
                // TODO: maybe we should use a singleton to deduplicate loading by
                // path? Probably not much use for duplicate SVG widgets however.
                let data = std::fs::read(&self.path).unwrap();

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
                    resources_dir: self.path.parent(),
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

                self.tree = Some(usvg::Tree::from_data(&data, &opts).unwrap());
            }
        }
    }

    impl Layout for Svg {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let size = self.tree.as_ref()
                .map(|tree| Size::from(tree.svg_node().size.to_screen_size().dimensions()))
                .unwrap_or(Size(128, 128));
            self.sprite.size_rules(size_mgr, axis, size)
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            self.core.rect = self.sprite.align_rect(rect, align, Size::ZERO);
            let size: (u32, u32) = self.core.rect.size.cast();

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
