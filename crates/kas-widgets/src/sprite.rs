// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! 2D pixmap widget

use kas::layout::{AspectScaling, SpriteDisplay};
use kas::prelude::*;
use std::path::PathBuf;

impl_scope! {
    /// An image with margins
    #[derive(Clone, Debug, Default)]
    #[widget]
    pub struct Image {
        #[widget_core]
        core: CoreData,
        sprite: SpriteDisplay,
        sprite_size: Size,
        path: PathBuf,
        do_load: bool,
        id: Option<ImageId>,
    }

    impl WidgetConfig for Image {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            if self.do_load {
                self.do_load = false;
                match mgr.draw_shared()
                        .image_from_path(&self.path)
                        .map(|id| (id, mgr.draw_shared().image_size(id).unwrap_or(Size::ZERO)))
                {
                    Ok((id, size)) => {
                        self.id = Some(id);
                        self.sprite_size = size;
                    }
                    Err(error) => self.handle_load_fail(&error),
                }
            }
        }
    }

    impl Layout for Image {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.sprite.size_rules(size_mgr, axis, self.sprite_size)
        }

        fn set_rect(&mut self, _: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            self.core_data_mut().rect = self.sprite.align_rect(rect, align, self.sprite_size);
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            if let Some(id) = self.id {
                draw.image(self, id);
            }
        }
    }
}

impl Image {
    /// Construct with a path
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Image {
            core: Default::default(),
            sprite: SpriteDisplay {
                aspect: AspectScaling::Fixed,
                ..Default::default()
            },
            sprite_size: Size::ZERO,
            path: path.into(),
            do_load: true,
            id: None,
        }
    }

    /// Adjust scaling
    ///
    /// By default, this is [`SpriteDisplay::default`] except with
    /// `aspect: AspectScaling::Fixed`.
    #[inline]
    #[must_use]
    pub fn with_scaling(mut self, f: impl FnOnce(&mut SpriteDisplay)) -> Self {
        f(&mut self.sprite);
        self
    }

    /// Adjust scaling
    ///
    /// By default, this is [`SpriteDisplay::default`] except with
    /// `aspect: AspectScaling::Fixed`.
    #[inline]
    pub fn set_scaling(&mut self, f: impl FnOnce(&mut SpriteDisplay)) -> TkAction {
        f(&mut self.sprite);
        // NOTE: if only `aspect` is changed, REDRAW is enough
        TkAction::RESIZE
    }

    /// Set image path
    pub fn set_path<P: Into<PathBuf>>(&mut self, mgr: &mut SetRectMgr, path: P) {
        self.path = path.into();
        self.do_load = false;
        let mut size = Size::ZERO;
        if let Some(id) = self.id {
            mgr.draw_shared().image_free(id);
        }
        match mgr.draw_shared().image_from_path(&self.path) {
            Ok(id) => {
                self.id = Some(id);
                size = mgr.draw_shared().image_size(id).unwrap_or(Size::ZERO);
            }
            Err(error) => self.handle_load_fail(&error),
        };
        *mgr |= TkAction::REDRAW;
        if size != self.sprite_size {
            self.sprite_size = size;
            *mgr |= TkAction::RESIZE;
        }
    }

    /// Remove image (set empty)
    pub fn clear(&mut self, mgr: &mut SetRectMgr) {
        if let Some(id) = self.id.take() {
            self.do_load = false;
            mgr.draw_shared().image_free(id);
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
