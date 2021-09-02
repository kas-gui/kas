// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! SVG widget

// TODO: error handling (unwrap)

use kas::draw::{ImageFormat, ImageId};
use kas::geom::Vec2;
use kas::layout::MarginSelector;
use kas::{event, prelude::*};
use std::path::PathBuf;
use tiny_skia::Pixmap;

/// An SVG image loaded from a path
#[cfg_attr(doc_cfg, doc(cfg(feature = "svg")))]
#[derive(Clone, Widget)]
#[widget(config = noauto)]
pub struct Svg {
    #[widget_core]
    core: CoreData,
    path: PathBuf,
    tree: Option<usvg::Tree>,
    margins: MarginSelector,
    min_size_factor: f32,
    ideal_size_factor: f32,
    min_size: Size,
    ideal_size: Size,
    stretch: Stretch,
    pixmap: Option<Pixmap>,
    image_id: Option<ImageId>,
}

impl std::fmt::Debug for Svg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Svg")
            .field("core", &self.core)
            .field("path", &self.path)
            .field("margins", &self.margins)
            .field("min_size_factor", &self.min_size_factor)
            .field("ideal_size_factor", &self.ideal_size_factor)
            .field("min_size", &self.min_size)
            .field("ideal_size", &self.ideal_size)
            .field("stretch", &self.stretch)
            .field("pixmap", &self.pixmap)
            .field("image_id", &self.image_id)
            .finish_non_exhaustive()
    }
}

impl Svg {
    /// Construct with a path and size factors
    ///
    /// An SVG image has an embedded "original" size. This constructor
    /// multiplies that size by the given factors to obtain minimum and ideal
    /// sizes (see [`SizeRules`] for a description of min / ideal sizes).
    pub fn from_path_and_factors<P: Into<PathBuf>>(
        path: P,
        min_size_factor: f32,
        ideal_size_factor: f32,
    ) -> Self {
        Svg {
            core: Default::default(),
            path: path.into(),
            tree: None,
            margins: MarginSelector::Outer,
            min_size_factor,
            ideal_size_factor,
            min_size: Size::ZERO,
            ideal_size: Size::ZERO,
            stretch: Stretch::Low,
            pixmap: None,
            image_id: None,
        }
    }

    /// Set margins
    pub fn with_margins(mut self, margins: MarginSelector) -> Self {
        self.margins = margins;
        self
    }

    /// Set stretch policy
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
    fn configure(&mut self, mgr: &mut Manager) {
        if self.tree.is_none() {
            // TODO: maybe we should use a singleton to deduplicate loading by
            // path? Probably not much use for duplicate SVG widgets however.
            let data = std::fs::read(&self.path).unwrap();

            let mut opts = usvg::Options::default();
            if let Some(path) = self.path.parent() {
                opts.resources_dir = Some(path.to_path_buf());
            }
            let scale_factor = mgr.scale_factor();
            opts.dpi = 96.0 * f64::conv(scale_factor);
            // TODO: allow configuration for rendering options (speed vs quality)
            // TODO: set font family and database

            let tree = usvg::Tree::from_data(&data, &opts.to_ref()).unwrap();
            let size = tree.svg_node().size.to_screen_size().dimensions();
            self.tree = Some(tree);
            let size = Vec2(size.0.cast(), size.1.cast());
            self.min_size = Size::from(size * self.min_size_factor * scale_factor);
            self.ideal_size = Size::from(size * self.ideal_size_factor * scale_factor);
        }
    }
}

impl Layout for Svg {
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let margins = self.margins.select(sh);
        if axis.is_horizontal() {
            SizeRules::new(
                self.min_size.0,
                self.ideal_size.0,
                margins.horiz,
                self.stretch,
            )
        } else {
            SizeRules::new(
                self.min_size.1,
                self.ideal_size.1,
                margins.vert,
                self.stretch,
            )
        }
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let size = match self.ideal_size.aspect_scale_to(rect.size) {
            Some(size) => {
                self.core_data_mut().rect = align
                    .complete(Align::Centre, Align::Centre)
                    .aligned_rect(size, rect);
                Into::<(u32, u32)>::into(size)
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
                mgr.draw_shared(|ds| ds.image_free(id));
            }
            self.pixmap = Pixmap::new(size.0, size.1);
            if let Some(tree) = self.tree.as_ref() {
                self.image_id = self.pixmap.as_mut().map(|pm| {
                    let (w, h) = (pm.width(), pm.height());

                    // alas, we cannot tell resvg to skip the aspect-ratio-scaling!
                    resvg::render(tree, usvg::FitTo::Height(h), pm.as_mut());

                    mgr.draw_shared(|ds| {
                        let id = ds.image_alloc((w, h)).unwrap();
                        ds.image_upload(id, pm.data(), ImageFormat::Rgba8);
                        id
                    })
                });
            }
        }
    }

    fn draw(&self, draw: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {
        if let Some(id) = self.image_id {
            draw.image(id, self.rect());
        }
    }
}
