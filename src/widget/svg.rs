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
#[derive(Clone, Widget)]
pub struct Svg {
    #[widget_core]
    core: CoreData,
    tree: usvg::Tree,
    margins: MarginSelector,
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
            .field("margins", &self.margins)
            .field("min_size", &self.min_size)
            .field("ideal_size", &self.ideal_size)
            .field("stretch", &self.stretch)
            .field("pixmap", &self.pixmap)
            .field("image_id", &self.image_id)
            .finish_non_exhaustive()
    }
}

impl Svg {
    /// Construct with a path
    pub fn new<P: Into<PathBuf>>(path: P, min_size_factor: f32, ideal_size_factor: f32) -> Self {
        // TODO: use resource manager for path deduplication and loading
        let mut path = path.into();
        let data = std::fs::read(&path).unwrap();

        let mut opts = usvg::Options::default();
        if path.pop() {
            opts.resources_dir = Some(path);
        }
        // TODO: set additional opts

        let tree = usvg::Tree::from_data(&data, &opts).unwrap();
        // TODO: this should be scaled by scale_factor?
        let size = tree.svg_node().size.to_screen_size().dimensions();
        let size = Vec2(size.0.cast(), size.1.cast());
        let min_size = Size::from(size * min_size_factor);
        let ideal_size = Size::from(size * ideal_size_factor);

        Svg {
            core: Default::default(),
            tree,
            margins: MarginSelector::Outer,
            min_size,
            ideal_size,
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
    pub fn margins(&mut self, margins: MarginSelector) {
        self.margins = margins;
    }

    /// Set stretch policy
    pub fn set_stretch(&mut self, stretch: Stretch) {
        self.stretch = stretch;
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

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, _align: AlignHints) {
        self.core_data_mut().rect = rect;
        let pm_size = self.pixmap.as_ref().map(|pm| (pm.width(), pm.height()));
        if Size::from(pm_size.unwrap_or((0, 0))) != rect.size {
            if let Some(id) = self.image_id {
                mgr.draw_shared(|ds| ds.remove_image(id));
            }
            self.pixmap = Pixmap::new(rect.size.0.cast(), rect.size.1.cast());
            let tree = &self.tree;
            self.image_id = self.pixmap.as_mut().map(|pm| {
                let (w, h) = (pm.width(), pm.height());
                resvg::render(tree, usvg::FitTo::Size(w, h), pm.as_mut());
                mgr.draw_shared(|ds| {
                    let id = ds.image_alloc((w, h)).unwrap();
                    ds.image_upload(id, pm.data(), ImageFormat::Rgba8);
                    id
                })
            });
        }
    }

    fn draw(&self, draw: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {
        if let Some(id) = self.image_id {
            draw.image(id, self.rect());
        }
    }
}
