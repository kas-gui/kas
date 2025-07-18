// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Canvas widget

use kas::draw::{ImageFormat, ImageHandle};
use kas::layout::{LogicalSize, PixmapScaling};
use kas::prelude::*;
use std::cell::RefCell;
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
    /// pixmap is created and resized and when requested by [`Self::need_redraw`].
    ///
    /// Note that [`Layout::draw`] does not call this method, but instead draws
    /// from a copy of the `pixmap` (updated each time this method completes).
    fn draw(&self, pixmap: &mut Pixmap);

    /// This method is called each time a frame is drawn. Note that since
    /// redrawing is async and non-blocking, the result is expected to be at
    /// least one frame late.
    ///
    /// The default implementation returns `false`.
    fn need_redraw(&mut self) -> bool {
        false
    }
}

async fn draw<P: CanvasProgram>(program: P, mut pixmap: Pixmap) -> (P, Pixmap) {
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
    /// Redraw if requested
    fn maybe_redraw(&mut self) -> Option<impl Future<Output = (P, Pixmap)> + use<P>> {
        if let State::Ready(p, _) = self
            && p.need_redraw()
            && let State::Ready(p, px) = std::mem::replace(self, State::Rendering)
        {
            return Some(draw(p, px));
        }

        None
    }

    /// Resize if required, redrawing on resize
    ///
    /// Returns a future to redraw. Does nothing if currently redrawing.
    fn resize(&mut self, (w, h): (u32, u32)) -> Option<impl Future<Output = (P, Pixmap)> + use<P>> {
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

#[impl_self]
mod Canvas {
    /// A canvas widget over the `tiny-skia` library
    ///
    /// The widget is essentially a cached image drawn from a [`Pixmap`]
    /// controlled through an implementation of [`CanvasProgram`].
    /// Note that the `tiny-skia` API is re-exported as [`crate::tiny_skia`].
    ///
    /// By default, a `Canvas` has a minimum size of 128x128 pixels and a high
    /// stretch factor (i.e. will greedily occupy extra space). To adjust this
    /// call one of the sizing/scaling methods.
    #[autoimpl(Debug ignore self.inner)]
    #[derive(Clone)]
    #[widget]
    pub struct Canvas<P: CanvasProgram> {
        core: widget_core!(),
        scaling: PixmapScaling,
        inner: RefCell<State<P>>,
        image: Option<ImageHandle>,
    }

    impl Self {
        /// Construct
        ///
        /// Use [`Self::with_size`] or [`Self::with_scaling`] to set the initial size.
        #[inline]
        pub fn new(program: P) -> Self {
            Canvas {
                core: Default::default(),
                scaling: PixmapScaling {
                    size: LogicalSize(128.0, 128.0),
                    stretch: Stretch::High,
                    ..Default::default()
                },
                inner: RefCell::new(State::Initial(program)),
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
        pub fn set_scaling(&mut self, cx: &mut EventState, f: impl FnOnce(&mut PixmapScaling)) {
            f(&mut self.scaling);
            // NOTE: if only `aspect` is changed, REDRAW is enough
            cx.resize(self);
        }
    }

    impl Layout for Self {
        fn rect(&self) -> Rect {
            self.scaling.rect
        }

        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            self.scaling.size_rules(sizer, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            let align = hints.complete_default();
            let scale_factor = cx.size_cx().scale_factor();
            self.scaling.set_rect(rect, align, scale_factor);

            let size = self.rect().size.cast();
            if let Some(fut) = self.inner.get_mut().resize(size) {
                cx.send_spawn(self.id(), fut);
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            if let Ok(mut state) = self.inner.try_borrow_mut()
                && let Some(fut) = state.maybe_redraw()
            {
                draw.ev_state().send_spawn(self.id(), fut);
            }

            if let Some(id) = self.image.as_ref().map(|h| h.id()) {
                draw.image(self.rect(), id);
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Canvas
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some((program, mut pixmap)) = cx.try_pop::<(P, Pixmap)>() {
                debug_assert!(matches!(self.inner.get_mut(), State::Rendering));
                let size = (pixmap.width(), pixmap.height());
                let ds = cx.draw_shared();

                if let Some(im_size) = self.image.as_ref().and_then(|h| ds.image_size(h))
                    && im_size != Size::conv(size)
                    && let Some(handle) = self.image.take()
                {
                    ds.image_free(handle);
                }

                if self.image.is_none() {
                    self.image = ds.image_alloc(size).ok();
                }

                if let Some(handle) = self.image.as_ref() {
                    ds.image_upload(handle, pixmap.data(), ImageFormat::Rgba8);
                }

                cx.redraw(&self);

                let rect_size: (u32, u32) = self.rect().size.cast();
                let state = self.inner.get_mut();
                if rect_size != size {
                    // Possible if a redraw was in progress when set_rect was called

                    pixmap = if let Some(px) = Pixmap::new(rect_size.0, rect_size.1) {
                        px
                    } else {
                        *state = State::Initial(program);
                        return;
                    };
                    cx.send_spawn(self.id(), draw(program, pixmap));
                } else {
                    *state = State::Ready(program, pixmap);
                }
            }
        }
    }
}
