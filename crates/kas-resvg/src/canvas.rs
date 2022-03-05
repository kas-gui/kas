// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Canvas widget

use kas::draw::{ImageFormat, ImageId};
use kas::layout::{SpriteDisplay, SpriteScaling};
use kas::prelude::*;
use tiny_skia::{Color, Pixmap};

/// Draws to a [`Canvas`]'s [`Pixmap`]
pub trait CanvasProgram: std::fmt::Debug + 'static {
    /// Draw image
    ///
    /// This method should draw an image to the canvas. It is called when the
    /// pixmap is created and resized, when [`Canvas::redraw`] is called, and
    /// when requested by [`CanvasProgram::do_redraw_animate`].
    fn draw(&mut self, pixmap: &mut Pixmap);

    /// On draw
    ///
    /// This is called just before each time the [`Canvas`] widget is drawn,
    /// and returns a tuple, `(redraw, animate)`:
    ///
    /// -   if `redraw`, then [`Self::draw`] is called
    /// -   if `animate`, then a new animation frame is requested after this one
    fn do_redraw_animate(&mut self) -> (bool, bool) {
        (false, false)
    }
}

widget! {
    /// A canvas widget over the `tiny-skia` library
    ///
    /// Note that the `tiny-skia` API is re-exported as [`crate::tiny_skia`].
    ///
    /// Canvas size is controlled by the sizing arguments passed to the constructor,
    /// as well as the `stretch` factor and the display's scale factor `sf`.
    /// Minimum size is `min_size * sf`. Ideal size is `ideal_size * sf` except that
    /// if `fix_aspect` is true, then the ideal height is the one that preserves
    /// aspect ratio for the given width. The canvas may also exceed the ideal size
    /// if a [`Stretch`] factor greater than `None` is used.
    ///
    /// The canvas (re)creates the backing pixmap when the size is set and draws
    /// to the new pixmap immediately. If the canvas program is modified then
    /// [`Canvas::redraw`] must be called to update the pixmap.
    #[cfg_attr(doc_cfg, doc(cfg(feature = "canvas")))]
    #[derive(Clone, Debug,)]
    pub struct Canvas<P: CanvasProgram> {
        #[widget_core]
        core: CoreData,
        sprite: SpriteDisplay,
        pixmap: Option<Pixmap>,
        image_id: Option<ImageId>,
        /// The program drawing to the canvas
        pub program: P,
    }

    impl Self {
        /// Construct with given size
        ///
        /// Creates a new canvas with a given size. The size is adjusted
        /// according to the display's scale factor.
        #[inline]
        pub fn new(program: P, size: Size) -> Self {
            Canvas {
                core: Default::default(),
                sprite: SpriteDisplay {
                    margins: Default::default(),
                    size,
                    scaling: SpriteScaling::Real,
                    aspect: Default::default(),
                    stretch: Stretch::High,
                },
                pixmap: None,
                image_id: None,
                program,
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

        /// Redraw immediately
        ///
        /// Other than this, the canvas is only drawn when the backing pixmap is
        /// (re)created: on start and on resizing.
        ///
        /// This method does nothing before a backing pixmap has been created.
        pub fn redraw(&mut self, mgr: &mut SetRectMgr) {
            if let Some((pm, id)) = self.pixmap.as_mut().zip(self.image_id) {
                pm.fill(Color::TRANSPARENT);
                self.program.draw(pm);
                mgr.draw_shared().image_upload(id, pm.data(), ImageFormat::Rgba8);
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.sprite.size_rules(size_mgr, axis)
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            self.core.rect = self.sprite.align_rect(rect, align);
            let size: (u32, u32) = self.core.rect.size.cast();

            let pm_size = self.pixmap.as_ref().map(|pm| (pm.width(), pm.height()));
            if pm_size.unwrap_or((0, 0)) != size {
                if let Some(id) = self.image_id {
                    mgr.draw_shared().image_free(id);
                }
                self.pixmap = Pixmap::new(size.0, size.1);
                let program = &mut self.program;
                self.image_id = self.pixmap.as_mut().map(|pm| {
                    program.draw(pm);
                    let (w, h) = (pm.width(), pm.height());
                    let id = mgr.draw_shared().image_alloc((w, h)).unwrap();
                    mgr.draw_shared().image_upload(id, pm.data(), ImageFormat::Rgba8);
                    id
                });
            }
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let mut draw = draw.with_id(self.id_ref());
            let (redraw, animate) = self.program.do_redraw_animate();
            if redraw {
                draw.set_rect_mgr(|mgr| self.redraw(mgr));
            }
            if animate {
                draw.draw_device().animate();
            }
            if let Some(id) = self.image_id {
                draw.image(id, self.rect());
            }
        }
    }
}
