// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! 2D pixmap widget

use kas::layout::SpriteDisplay;
use kas::{event, prelude::*};
use std::path::PathBuf;

widget! {
    /// An image with margins
    #[derive(Clone, Debug, Default)]
    pub struct Image {
        #[widget_core]
        core: CoreData,
        sprite: SpriteDisplay,
        path: PathBuf,
        do_load: bool,
        id: Option<ImageId>,
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
}

impl Image {
    /// Construct with a path
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Image {
            core: Default::default(),
            sprite: Default::default(),
            path: path.into(),
            do_load: true,
            id: None,
        }
    }

    /// Adjust scaling
    #[inline]
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
