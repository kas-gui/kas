// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Image widget

use kas::geom::Vec2;
use kas::{event, prelude::*};
use std::path::PathBuf;

/// Image scaling policies
#[derive(Clone, Copy, Debug)]
pub enum ImageScaling {
    /// No scaling; align in available space
    None,
    /// Fixed aspect ratio scaling; align on other axis
    FixedAspect,
    /// Stretch on both axes without regard for aspect ratio
    Stretch,
}

impl Default for ImageScaling {
    fn default() -> Self {
        ImageScaling::FixedAspect
    }
}

/// An image with margins
///
/// TODO: `BareImage` variant without margins
#[derive(Clone, Debug, Default, Widget)]
#[widget(config = noauto)]
pub struct Image {
    #[widget_core]
    core: CoreData,
    path: PathBuf,
    do_load: bool,
    id: Option<ImageId>,
    img_size: Size,
    scaling: ImageScaling,
    stretch: Stretch,
}

impl Image {
    /// Construct with a path
    ///
    /// TODO: low level variant allowing use of an existing image resource?
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Image {
            core: Default::default(),
            path: path.into(),
            do_load: true,
            id: None,
            img_size: Size::ZERO,
            scaling: Default::default(),
            stretch: Stretch::None,
        }
    }

    /// Set scaling mode
    pub fn with_scaling(mut self, scaling: ImageScaling) -> Self {
        self.scaling = scaling;
        self
    }

    /// Set stretch policy
    pub fn with_stretch(mut self, stretch: Stretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// Set scaling mode
    pub fn set_scaling(&mut self, scaling: ImageScaling) {
        self.scaling = scaling;
    }

    /// Set stretch policy
    pub fn set_stretch(&mut self, stretch: Stretch) {
        self.stretch = stretch;
    }

    /// Set image path
    pub fn set_path<P: Into<PathBuf>>(&mut self, mgr: &mut Manager, path: P) {
        self.path = path.into();
        self.do_load = false;
        let mut img_size = Size::ZERO;
        mgr.size_handle(|sh| {
            if let Some(id) = self.id {
                sh.remove_image(id);
            }
            match sh.load_image(&self.path) {
                Ok(id) => {
                    self.id = Some(id);
                    img_size = sh.image(id).unwrap_or(Size::ZERO);
                }
                Err(error) => self.handle_load_fail(&*error),
            };
        });
        mgr.redraw(self.id());
        if img_size != self.img_size {
            *mgr |= TkAction::RESIZE;
        }
    }

    /// Remove image (set empty)
    pub fn clear(&mut self, mgr: &mut Manager) {
        if let Some(id) = self.id.take() {
            self.do_load = false;
            mgr.size_handle(|sh| sh.remove_image(id));
        }
    }

    fn handle_load_fail(&mut self, mut error: &(dyn std::error::Error)) {
        self.id = None;
        log::warn!("Failed to load image: {}", self.path.display());
        loop {
            log::warn!("Cause: {}", error);
            if let Some(source) = error.source() {
                error = source;
            } else {
                break;
            }
        }
    }
}

impl WidgetConfig for Image {
    fn configure(&mut self, mgr: &mut Manager) {
        if self.do_load {
            self.do_load = false;
            match mgr.size_handle(|sh| sh.load_image(&self.path)) {
                Ok(id) => self.id = Some(id),
                Err(error) => self.handle_load_fail(&*error),
            }
        }
    }
}

impl Layout for Image {
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.img_size = self.id.and_then(|id| sh.image(id)).unwrap_or(Size::ZERO);
        let margins = sh.outer_margins();
        SizeRules::extract(axis, self.img_size, margins, self.stretch)
    }

    fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
        let ideal = match self.scaling {
            ImageScaling::None => self.img_size,
            ImageScaling::FixedAspect => {
                let img_size = Vec2::from(self.img_size);
                let ratio = Vec2::from(rect.size) / img_size;
                // Use smaller ratio, which must be finite
                if ratio.0 < ratio.1 {
                    Size(rect.size.0, i32::conv_nearest(ratio.0 * img_size.1))
                } else if ratio.1 < ratio.0 {
                    Size(i32::conv_nearest(ratio.1 * img_size.0), rect.size.1)
                } else {
                    // Non-finite ratio implies img_size is zero on at least one axis
                    rect.size
                }
            }
            ImageScaling::Stretch => rect.size,
        };
        let rect = align
            .complete(Default::default(), Default::default())
            .aligned_rect(ideal, rect);
        self.core_data_mut().rect = rect;
    }

    fn draw(&self, draw: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {
        if let Some(id) = self.id {
            draw.image(id, self.rect());
        }
    }
}
