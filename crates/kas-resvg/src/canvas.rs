// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Canvas widget

use kas::draw::{ImageFormat, ImageId};
use kas::layout::{SpriteDisplay, SpriteScaling};
use kas::{event, prelude::*};
use tiny_skia::{Color, Pixmap};

/// Draws to a [`Canvas`]'s [`Pixmap`]
pub trait CanvasDrawable: std::fmt::Debug + 'static {
    /// Draw
    ///
    /// This is called whenever the [`Pixmap`] is resized. One should check the
    /// pixmap's dimensions and scale the contents appropriately.
    fn draw(&self, pixmap: &mut Pixmap);
}

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
#[derive(Clone, Debug, Widget)]
pub struct Canvas<P: CanvasDrawable> {
    #[widget_core]
    core: CoreData,
    sprite: SpriteDisplay,
    pixmap: Option<Pixmap>,
    image_id: Option<ImageId>,
    /// The program drawing to the canvas
    pub program: P,
}

impl<P: CanvasDrawable> Canvas<P> {
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
    pub fn redraw(&mut self, mgr: &mut Manager) {
        if let Some((pm, id)) = self.pixmap.as_mut().zip(self.image_id) {
            pm.fill(Color::TRANSPARENT);
            self.program.draw(pm);
            mgr.draw_shared(|ds| ds.image_upload(id, pm.data(), ImageFormat::Rgba8));
        }
    }
}

impl<P: CanvasDrawable> Layout for Canvas<P> {
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.sprite.size_rules(sh, axis)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = self.sprite.align_rect(rect, align);
        let size: (u32, u32) = self.core.rect.size.into();

        let pm_size = self.pixmap.as_ref().map(|pm| (pm.width(), pm.height()));
        if pm_size.unwrap_or((0, 0)) != size {
            if let Some(id) = self.image_id {
                mgr.draw_shared(|ds| ds.image_free(id));
            }
            self.pixmap = Pixmap::new(size.0, size.1);
            let program = &self.program;
            self.image_id = self.pixmap.as_mut().map(|pm| {
                program.draw(pm);
                mgr.draw_shared(|ds| {
                    let (w, h) = (pm.width(), pm.height());
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