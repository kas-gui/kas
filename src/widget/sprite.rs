// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! 2D pixmap widget

use kas::geom::Vec2;
use kas::layout::MarginSelector;
use kas::{event, prelude::*};
use std::path::PathBuf;

/// Scaling policies
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SpriteScaling {
    /// No scaling; align in available space
    None,
    /// Fixed aspect ratio scaling; align on other axis
    FixedAspect,
    /// Stretch on both axes without regard for aspect ratio
    Stretch,
}

impl Default for SpriteScaling {
    fn default() -> Self {
        SpriteScaling::FixedAspect
    }
}

/// Widget component for displaying a sprite
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpriteDisplay {
    /// Margins
    pub margins: MarginSelector,
    /// The native size of the sprite
    pub size: Size,
    /// Widget stretchiness
    pub stretch: Stretch,
    /// Sprite scaling mode
    pub scaling: SpriteScaling,
}

impl SpriteDisplay {
    /// Generates `size_rules` based on size
    ///
    /// Set [`Self::size`] before calling this.
    pub fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let margins = self.margins.select(sh);
        SizeRules::extract(axis, self.size, margins, self.stretch)
    }

    /// Aligns `rect` according to stretch policy
    ///
    /// Assign the result to `self.core_data_mut().rect`.
    pub fn align_rect(&mut self, rect: Rect, align: AlignHints) -> Rect {
        let ideal = match self.scaling {
            SpriteScaling::None => self.size,
            SpriteScaling::FixedAspect => {
                let size = Vec2::from(self.size);
                let ratio = Vec2::from(rect.size) / size;
                // Use smaller ratio, which must be finite
                if ratio.0 < ratio.1 {
                    Size(rect.size.0, i32::conv_nearest(ratio.0 * size.1))
                } else if ratio.1 < ratio.0 {
                    Size(i32::conv_nearest(ratio.1 * size.0), rect.size.1)
                } else {
                    // Non-finite ratio implies size is zero on at least one axis
                    rect.size
                }
            }
            SpriteScaling::Stretch => rect.size,
        };
        align
            .complete(Default::default(), Default::default())
            .aligned_rect(ideal, rect)
    }
}

/// An image with margins
#[derive(Clone, Debug, Default, Widget)]
#[widget(config = noauto)]
pub struct Image {
    #[widget_core]
    core: CoreData,
    sprite: SpriteDisplay,
    path: PathBuf,
    do_load: bool,
    id: Option<ImageId>,
}

impl Image {
    /// Construct with a path
    ///
    /// TODO: low level variant allowing use of an existing image resource?
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Image {
            core: Default::default(),
            sprite: Default::default(),
            path: path.into(),
            do_load: true,
            id: None,
        }
    }

    /// Set margins
    pub fn with_margins(mut self, margins: MarginSelector) -> Self {
        self.sprite.margins = margins;
        self
    }

    /// Set scaling mode
    pub fn with_scaling(mut self, scaling: SpriteScaling) -> Self {
        self.sprite.scaling = scaling;
        self
    }

    /// Set stretch policy
    pub fn with_stretch(mut self, stretch: Stretch) -> Self {
        self.sprite.stretch = stretch;
        self
    }

    /// Set margins
    pub fn margins(&mut self, margins: MarginSelector) {
        self.sprite.margins = margins;
    }

    /// Set scaling mode
    pub fn set_scaling(&mut self, scaling: SpriteScaling) {
        self.sprite.scaling = scaling;
    }

    /// Set stretch policy
    pub fn set_stretch(&mut self, stretch: Stretch) {
        self.sprite.stretch = stretch;
    }

    /// Set image path
    pub fn set_path<P: Into<PathBuf>>(&mut self, mgr: &mut Manager, path: P) {
        self.path = path.into();
        self.do_load = false;
        let mut size = Size::ZERO;
        mgr.draw_shared(|ds| {
            if let Some(id) = self.id {
                ds.image_free(id);
            }
            match ds.image_from_path(&self.path) {
                Ok(id) => {
                    self.id = Some(id);
                    size = ds.image_size(id).unwrap_or(Size::ZERO);
                }
                Err(error) => self.handle_load_fail(&error),
            };
        });
        mgr.redraw(self.id());
        if size != self.sprite.size {
            self.sprite.size = size;
            *mgr |= TkAction::RESIZE;
        }
    }

    /// Remove image (set empty)
    pub fn clear(&mut self, mgr: &mut Manager) {
        if let Some(id) = self.id.take() {
            self.do_load = false;
            mgr.draw_shared(|ds| ds.image_free(id));
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
            match mgr.draw_shared(|ds| {
                ds.image_from_path(&self.path)
                    .map(|id| (id, ds.image_size(id).unwrap_or(Size::ZERO)))
            }) {
                Ok((id, size)) => {
                    self.id = Some(id);
                    self.sprite.size = size;
                }
                Err(error) => self.handle_load_fail(&error),
            }
        }
    }
}

impl Layout for Image {
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.sprite.size_rules(sh, axis)
    }

    fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
        self.core_data_mut().rect = self.sprite.align_rect(rect, align);
    }

    fn draw(&self, draw: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {
        if let Some(id) = self.id {
            draw.image(id, self.rect());
        }
    }
}
