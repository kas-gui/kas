// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Canvas widget

use kas::draw::{ImageFormat, ImageId};
use kas::layout::MarginSelector;
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
/// Note that the `tiny-skia` API is re-exported as [`kas::widget::tiny_skia`].
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
    margins: MarginSelector,
    fix_aspect: bool,
    min_size: Size,
    ideal_size: Size,
    stretch: Stretch,
    pixmap: Option<Pixmap>,
    image_id: Option<ImageId>,
    /// The program drawing to the canvas
    pub program: P,
}

impl<P: CanvasDrawable> Canvas<P> {
    /// Construct with given size
    ///
    /// Creates a new canvas with `min_size = ideal_size = size` and
    /// `fix_aspect = true`. Use [`Canvas::new_sizes`] for more control.
    /// Sizes are multiplied by the display's scale factor.
    #[inline]
    pub fn new(program: P, size: Size) -> Self {
        Self::new_sizes(program, size, size, true)
    }

    /// Construct with given sizes and optional aspect ratio fixing
    ///
    /// Sizes are multiplied by the display's scale factor. See [`Canvas`]
    /// documentation for more details on sizing.
    pub fn new_sizes(program: P, min_size: Size, ideal_size: Size, fix_aspect: bool) -> Self {
        Canvas {
            core: Default::default(),
            margins: MarginSelector::Outer,
            fix_aspect,
            min_size,
            ideal_size,
            stretch: Stretch::None,
            pixmap: None,
            image_id: None,
            program,
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
        let margins = self.margins.select(sh);
        if axis.is_horizontal() {
            let mut ideal_w = self.ideal_size.0;
            if let Some(height) = axis.other() {
                ideal_w = i32::conv(
                    i64::conv(height) * i64::conv(ideal_w) / i64::conv(self.ideal_size.1),
                );
            }
            SizeRules::new(self.min_size.0, ideal_w, margins.horiz, self.stretch)
        } else {
            let mut ideal_h = self.ideal_size.1;
            if let Some(width) = axis.other() {
                ideal_h =
                    i32::conv(i64::conv(width) * i64::conv(ideal_h) / i64::conv(self.ideal_size.0));
            }
            SizeRules::new(self.min_size.1, ideal_h, margins.vert, self.stretch)
        }
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let size = if self.fix_aspect {
            match self.ideal_size.aspect_scale_to(rect.size) {
                Some(size) => {
                    self.core_data_mut().rect = align
                        .complete(Align::Centre, Align::Centre)
                        .aligned_rect(size, rect);
                    size
                }
                None => {
                    self.core_data_mut().rect = rect;
                    self.pixmap = None;
                    self.image_id = None;
                    return;
                }
            }
        } else {
            self.core_data_mut().rect = rect;
            rect.size
        };
        let size: (u32, u32) = size.into();

        let pm_size = self.pixmap.as_ref().map(|pm| (pm.width(), pm.height()));
        if pm_size.unwrap_or((0, 0)) != size {
            if let Some(id) = self.image_id {
                mgr.draw_shared(|ds| ds.remove_image(id));
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
