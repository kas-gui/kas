// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Canvas widget

use kas::draw::{ImageFormat, ImageHandle};
use kas::layout::{LogicalSize, PixmapScaling};
use kas::prelude::*;
use std::future::Future;
use tiny_skia::{Color, Pixmap};

/// Draws to a [`Canvas`]'s [`Pixmap`]
///
/// Note: the value is sometimes moved between threads, hence [`Send`] bound.
/// If the type is large it should be boxed.
pub trait CanvasProgram: std::fmt::Debug + Send + 'static {
    /// Draw image
    ///
    /// This method should draw an image to the canvas. It is called when the
    /// pixmap is created and resized, when [`Canvas::redraw`] is called, and
    /// when requested by [`CanvasProgram::do_redraw_animate`].
    fn draw(&mut self, pixmap: &mut Pixmap);

    /// This method is called after each draw completes. If it returns `true`
    /// a redraw is scheduled.
    ///
    /// TODO: how to animate with limited framerate?
    ///
    /// The default implementation returns `false`.
    fn need_redraw(&mut self) -> bool {
        false
    }
}

async fn draw<P: CanvasProgram>(mut program: P, mut pixmap: Pixmap) -> (P, Pixmap) {
    pixmap.fill(Color::TRANSPARENT);
    program.draw(&mut pixmap);
    (program, pixmap)
}

#[derive(Clone)]
enum State<P: CanvasProgram> {
    Initial(P),
    Rendering,
    Ready(P, Pixmap),
}

impl<P: CanvasProgram> State<P> {
    /// Resize if required, redrawing on resize
    ///
    /// Returns a future to redraw. Does nothing if currently redrawing.
    fn resize(&mut self, (w, h): (u32, u32)) -> Option<impl Future<Output = (P, Pixmap)>> {
        let old_state = std::mem::replace(self, State::Rendering);
        let (program, pixmap) = match old_state {
            State::Ready(p, px) if (px.width(), px.height()) == (w, h) => {
                *self = State::Ready(p, px);
                return None;
            }
            State::Rendering => return None,
            State::Initial(p) | State::Ready(p, _) => {
                if let Some(px) = Pixmap::new(w, h) {
                    (p, px)
                } else {
                    *self = State::Initial(p);
                    return None;
                }
            }
        };

        Some(draw(program, pixmap))
    }
}

impl_scope! {
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
    #[autoimpl(Debug ignore self.inner)]
    #[derive(Clone)]
    #[widget]
    pub struct Canvas<P: CanvasProgram> {
        core: widget_core!(),
        scaling: PixmapScaling,
        inner: State<P>,
        image: Option<ImageHandle>,
    }

    impl Self {
        /// Construct
        ///
        /// Use [`Self::with_size`] or [`Self::with_scaling`] to set the initial size.
        #[inline]
        pub fn new(program: P) -> Self {
            let mut scaling = PixmapScaling::default();
            scaling.size = LogicalSize(128.0, 128.0);
            scaling.stretch = Stretch::High;

            Canvas {
                core: Default::default(),
                scaling,
                inner: State::Initial(program),
                image: None,
            }
        }

        /// Assign size
        ///
        /// Default size is 128 × 128.
        #[inline]
        #[must_use]
        pub fn with_size(mut self, size: LogicalSize) -> Self {
            self.scaling.size = size;
            self
        }

        /// Adjust scaling
        ///
        /// Default size is 128 × 128; default stretch is [`Stretch::High`].
        /// Other fields use [`PixmapScaling`]'s default values.
        #[inline]
        #[must_use]
        pub fn with_scaling(mut self, f: impl FnOnce(&mut PixmapScaling)) -> Self {
            f(&mut self.scaling);
            self
        }

        /// Adjust scaling
        ///
        /// Default size is 128 × 128; default stretch is [`Stretch::High`].
        /// Other fields use [`PixmapScaling`]'s default values.
        #[inline]
        pub fn set_scaling(&mut self, f: impl FnOnce(&mut PixmapScaling)) -> Action {
            f(&mut self.scaling);
            // NOTE: if only `aspect` is changed, REDRAW is enough
            Action::RESIZE
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.scaling.size_rules(size_mgr, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            let scale_factor = mgr.size_mgr().scale_factor();
            self.core.rect = self.scaling.align_rect(rect, scale_factor);
            let size = self.core.rect.size.cast();

            if let Some(fut) = self.inner.resize(size) {
                mgr.push_spawn(self.id(), fut);
            }
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            if let Some(id) = self.image.as_ref().map(|h| h.id()) {
                draw.image(self.rect(), id);
            }
        }
    }

    impl Widget for Self {
        fn handle_message(&mut self, mgr: &mut EventMgr) {
            if let Some((mut program, mut pixmap)) = mgr.try_pop::<(P, Pixmap)>() {
                debug_assert!(matches!(self.inner, State::Rendering));
                let (w, h) = (pixmap.width(), pixmap.height());
                let size = Size::conv((w, h));

                mgr.draw_shared(|ds| {
                    if let Some(im_size) = self.image.as_ref().and_then(|h| ds.image_size(h)) {
                        if im_size != size {
                            if let Some(handle) = self.image.take() {
                                ds.image_free(handle);
                            }
                        }
                    }

                    if self.image.is_none() {
                        self.image = ds.image_alloc((w, h)).ok();
                    }

                    if let Some(handle) = self.image.as_ref() {
                        ds.image_upload(&handle, pixmap.data(), ImageFormat::Rgba8);
                    }
                });

                // API says we call this after every redraw:
                let mut need_redraw = program.need_redraw();
                mgr.redraw(self.id());

                if self.rect().size != size {
                    // Possible if a redraw was in progress when set_rect was called

                    pixmap = if let Some(px) = Pixmap::new(w, h) {
                        px
                    } else {
                        self.inner = State::Initial(program);
                        return;
                    };
                    need_redraw = true;
                }

                if need_redraw {
                    mgr.push_spawn(self.id(), draw(program, pixmap));
                } else {
                    self.inner = State::Ready(program, pixmap);
                }
            }
        }
    }
}
