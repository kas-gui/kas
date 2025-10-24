// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! SVG widget

use super::Scaling;
use kas::draw::{ImageFormat, ImageHandle};
use kas::layout::LogicalSize;
use kas::prelude::*;
use kas::theme::MarginStyle;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tiny_skia::{Pixmap, Transform};
use usvg::Tree;

/// Load errors
#[derive(thiserror::Error, Debug)]
enum LoadError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("SVG error")]
    Svg(#[from] usvg::Error),
}

fn load(data: &[u8], resources_dir: Option<&Path>) -> Result<Tree, usvg::Error> {
    use once_cell::sync::Lazy;
    static FONT_FAMILY: Lazy<String> = Lazy::new(|| {
        let mut resolver = kas::text::fonts::library().resolver();
        resolver
            .font_family_from_generic(kas::text::fonts::GenericFamily::Serif)
            .map(|s| s.to_string())
            .unwrap_or_default()
    });

    // Defaults are taken from usvg::Options::default(). Notes:
    // - adjusting for screen scale factor is purely a property of
    //   making the canvas larger and not important here
    // - default_size: affected by screen scale factor later
    // - dpi: according to css-values-3, 1in = 96px
    // - font_size: units are (logical) px per em; 16px = 12pt
    // - TODO: add option to clone fontdb from kas::text?
    let opts = usvg::Options {
        resources_dir: resources_dir.map(|path| path.to_owned()),
        dpi: 96.0,
        font_family: FONT_FAMILY.clone(),
        font_size: 16.0, // units: "logical pixels" per Em
        languages: vec!["en".to_string()],
        shape_rendering: usvg::ShapeRendering::default(),
        text_rendering: usvg::TextRendering::default(),
        image_rendering: usvg::ImageRendering::default(),
        default_size: usvg::Size::from_wh(100.0, 100.0).unwrap(),
        image_href_resolver: Default::default(),
        font_resolver: Default::default(),
        fontdb: Default::default(),
        style_sheet: None,
    };

    let tree = Tree::from_data(data, &opts)?;

    Ok(tree)
}

#[derive(Clone)]
enum Source {
    Static(&'static [u8], Option<PathBuf>),
    Heap(Arc<[u8]>, Option<PathBuf>),
}
impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Static(_, path) => write!(f, "Source::Static(_, {path:?}"),
            Source::Heap(_, path) => write!(f, "Source::Heap(_, {path:?}"),
        }
    }
}
impl Source {
    fn tree(&self) -> Result<Tree, usvg::Error> {
        let (data, res_dir) = match self {
            Source::Static(d, p) => (*d, p.as_ref()),
            Source::Heap(d, p) => (&**d, p.as_ref()),
        };
        load(data, res_dir.map(|p| p.as_ref()))
    }
}

#[derive(Clone, Debug, Default)]
enum State {
    #[default]
    None,
    Initial(Source),
    Rendering(Source),
    Ready(Source, Pixmap),
}

async fn draw(svg: Source, mut pixmap: Pixmap) -> Pixmap {
    if let Ok(tree) = svg.tree() {
        let w = f32::conv(pixmap.width()) / tree.size().width();
        let h = f32::conv(pixmap.height()) / tree.size().height();
        let transform = Transform::from_scale(w, h);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
    }
    pixmap
}

impl State {
    /// Resize if required, redrawing on resize
    ///
    /// Returns a future to redraw. Does nothing if currently redrawing.
    fn resize(&mut self, (w, h): (u32, u32)) -> Option<impl Future<Output = Pixmap> + use<>> {
        let old_state = std::mem::replace(self, State::None);
        match old_state {
            State::None => (),
            state @ State::Rendering(_) => *self = state,
            State::Ready(svg, px) if (px.width(), px.height()) == (w, h) => {
                *self = State::Ready(svg, px);
                return None;
            }
            State::Initial(svg) | State::Ready(svg, _) => {
                if let Some(px) = Pixmap::new(w, h) {
                    *self = State::Rendering(svg.clone());
                    return Some(draw(svg, px));
                } else {
                    *self = State::Initial(svg);
                    return None;
                }
            }
        }
        None
    }
}

#[impl_self]
mod Svg {
    /// An SVG image widget
    ///
    /// May be default constructed (result is empty).
    ///
    /// The size of the widget is inferred from the SVG source in logical pixels
    /// then scaled by the display's scale factor. If a different size should be
    /// used it must be set after loading the SVG data.
    ///
    /// By default, the drawn SVG will not be allowed to scale above its
    /// specified size; if the widget is forced to stretch, content will be
    /// positioned within this space according to alignment rules (centered by
    /// default).
    #[autoimpl(Debug ignore self.inner)]
    #[derive(Clone, Default)]
    #[widget]
    pub struct Svg {
        core: widget_core!(),
        inner: State,
        scaling: Scaling,
        image: Option<ImageHandle>,
    }

    impl Self {
        /// Construct from data
        ///
        /// Returns an error if the SVG fails to parse. If using this method
        /// with [`include_bytes`] it is probably safe to unwrap.
        ///
        /// The (logical) size of the widget is set to that from the SVG source.
        pub fn new(data: &'static [u8]) -> Result<Self, impl std::error::Error> {
            let mut svg = Svg::default();
            let source = Source::Static(data, None);
            svg.load_source(source).map(|_| svg)
        }

        /// Construct from a path
        pub fn new_path<P: AsRef<Path>>(path: P) -> Result<Self, impl std::error::Error> {
            let mut svg = Svg::default();
            svg._load_path(path.as_ref())?;
            Result::<Self, LoadError>::Ok(svg)
        }

        /// Load from `data`
        ///
        /// Replaces existing data and request a resize. The size is inferred
        /// from the SVG using units of logical pixels.
        pub fn load(
            &mut self,
            cx: &mut EventState,
            data: &'static [u8],
            resources_dir: Option<&Path>,
        ) -> Result<(), impl std::error::Error + use<>> {
            let source = Source::Static(data, resources_dir.map(|p| p.to_owned()));
            self.load_source(source).map(|_| cx.resize(self))
        }

        fn load_source(&mut self, source: Source) -> Result<(), usvg::Error> {
            // Set scaling size. TODO: this is useless if Self::with_size is called after.
            let size = source.tree()?.size();
            self.scaling.size = LogicalSize(size.width(), size.height());

            self.inner = match std::mem::take(&mut self.inner) {
                State::Ready(_, px) => State::Ready(source, px),
                _ => State::Initial(source),
            };
            Ok(())
        }

        /// Load from a path
        ///
        /// This is a wrapper around [`Self::load`].
        pub fn load_path<P: AsRef<Path>>(
            &mut self,
            cx: &mut EventState,
            path: P,
        ) -> Result<(), impl std::error::Error + use<P>> {
            self._load_path(path.as_ref()).map(|_| cx.resize(self))
        }

        fn _load_path(&mut self, path: &Path) -> Result<(), LoadError> {
            let buf = std::fs::read(path)?;
            let rd = path.parent().map(|path| path.to_owned());
            let source = Source::Heap(buf.into(), rd);
            Ok(self.load_source(source)?)
        }

        /// Set size in logical pixels
        pub fn set_logical_size(&mut self, size: impl Into<LogicalSize>) {
            self.scaling.size = size.into();
        }

        /// Set size in logical pixels (inline)
        #[must_use]
        pub fn with_logical_size(mut self, size: impl Into<LogicalSize>) -> Self {
            self.scaling.size = size.into();
            self
        }

        /// Set the margin style (inline)
        ///
        /// By default, this is [`MarginStyle::Large`].
        #[must_use]
        #[inline]
        pub fn with_margin_style(mut self, style: MarginStyle) -> Self {
            self.scaling.margins = style;
            self
        }

        /// Control whether the aspect ratio is fixed (inline)
        ///
        /// By default this is fixed.
        #[must_use]
        #[inline]
        pub fn with_fixed_aspect_ratio(mut self, fixed: bool) -> Self {
            self.scaling.fix_aspect = fixed;
            self
        }

        /// Set the stretch factor (inline)
        ///
        /// By default this is [`Stretch::None`]. Particular to this widget,
        /// [`Stretch::None`] will avoid stretching of content, aligning instead.
        #[must_use]
        #[inline]
        pub fn with_stretch(mut self, stretch: Stretch) -> Self {
            self.scaling.stretch = stretch;
            self
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            self.scaling.size_rules(cx, axis)
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            let align = hints.complete_default();
            let scale_factor = cx.scale_factor();
            let rect = self.scaling.align(rect, align, scale_factor);
            widget_set_rect!(rect);

            let size: (u32, u32) = self.rect().size.cast();
            if let Some(fut) = self.inner.resize(size) {
                cx.send_spawn(self.id(), fut);
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            if let Some(id) = self.image.as_ref().map(|h| h.id()) {
                draw.image(self.rect(), id);
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Image
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(pixmap) = cx.try_pop::<Pixmap>() {
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
                self.inner = match std::mem::take(&mut self.inner) {
                    State::None => State::None,
                    State::Initial(source) | State::Rendering(source) | State::Ready(source, _) => {
                        State::Ready(source, pixmap)
                    }
                };

                let own_size: (u32, u32) = self.rect().size.cast();
                if size != own_size
                    && let Some(fut) = self.inner.resize(own_size)
                {
                    cx.send_spawn(self.id(), fut);
                }
            }
        }
    }
}
