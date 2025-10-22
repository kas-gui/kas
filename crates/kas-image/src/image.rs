// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! 2D pixmap widget

use super::Sprite;
use image::{ImageError, ImageReader, RgbaImage};
use kas::layout::LogicalSize;
use kas::prelude::*;
use kas::theme::MarginStyle;
use kas::util::warn_about_error;
use std::path::{Path, PathBuf};

#[autoimpl(Debug ignore self.1)]
struct SetImage(PathBuf, Option<RgbaImage>);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum State {
    #[default]
    Empty,
    Loading,
    Loaded,
}

#[impl_self]
mod Image {
    /// A raster image widget, loaded from a path
    ///
    /// If your image source is not a path, use [`Sprite`] instead. Note also
    /// that since image-loading can be CPU- and IO-intensive, if images get
    /// reloaded frequently, you might benefit from using [`Sprite`] over a
    /// custom image loader with a cache.
    ///
    /// Size is inferred from the loaded image. By default, scaling is limited
    /// to integer multiples of the source image size.
    ///
    /// May be default constructed (result is empty).
    #[derive(Clone, Debug, Default)]
    #[widget]
    #[layout(self.raw)]
    pub struct Image {
        core: widget_core!(),
        #[widget]
        raw: Sprite,
        path: PathBuf,
        state: State,
    }

    impl Self {
        /// Construct with a given `path`
        ///
        /// The image will be loaded when the widget is configured.
        #[inline]
        pub fn new(path: impl Into<PathBuf>) -> Self {
            Image {
                core: Default::default(),
                raw: Sprite::default(),
                path: path.into(),
                state: State::Empty,
            }
        }

        /// Remove image (set empty)
        pub fn clear(&mut self, cx: &mut ConfigCx) {
            self.path.clear();
            self.state = State::Loading;
            cx.send(self.id(), SetImage(PathBuf::new(), None));
        }

        /// Set path and load image
        pub fn set(&mut self, cx: &mut ConfigCx, path: &Path) {
            if self.path == path {
                return;
            }

            self.path = path.to_path_buf();
            self.state = State::Empty;
            self.configure(cx);
        }

        /// Set size in logical pixels
        ///
        /// This enables fractional scaling of the image with a fixed aspect ratio.
        pub fn set_logical_size(&mut self, size: impl Into<LogicalSize>) {
            self.raw.set_logical_size(size);
        }

        /// Set size in logical pixels (inline)
        ///
        /// This enables fractional scaling of the image with a fixed aspect ratio.
        #[must_use]
        pub fn with_logical_size(mut self, size: impl Into<LogicalSize>) -> Self {
            self.raw.set_logical_size(size);
            self
        }

        /// Set the margin style (inline)
        ///
        /// By default, this is [`MarginStyle::Large`].
        #[must_use]
        #[inline]
        pub fn with_margin_style(mut self, style: MarginStyle) -> Self {
            self.raw = self.raw.with_margin_style(style);
            self
        }

        /// Control whether the aspect ratio is fixed (inline)
        ///
        /// This is only applicable when using fractional scaling (see
        /// [`Self::set_logical_size`]) since integer scaling always uses a
        /// fixed aspect ratio. By default this is enabled.
        #[must_use]
        #[inline]
        pub fn with_fixed_aspect_ratio(mut self, fixed: bool) -> Self {
            self.raw = self.raw.with_fixed_aspect_ratio(fixed);
            self
        }

        /// Set the stretch factor (inline)
        ///
        /// By default this is [`Stretch::None`]. Particular to this widget,
        /// [`Stretch::None`] will avoid stretching of content, aligning instead.
        #[must_use]
        #[inline]
        pub fn with_stretch(mut self, stretch: Stretch) -> Self {
            self.raw = self.raw.with_stretch(stretch);
            self
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Image
        }
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            if self.state == State::Empty && !self.path.as_os_str().is_empty() {
                self.state = State::Loading;
                let path = self.path.clone();
                cx.send_spawn(self.id(), async {
                    let result = ImageReader::open(&path)
                        .and_then(|reader| reader.with_guessed_format())
                        .map_err(|err| ImageError::IoError(err))
                        .and_then(|reader| reader.decode())
                        .map(|image| image.into_rgba8())
                        .inspect_err(|err| warn_about_error("Failed to read image", err))
                        .ok();

                    SetImage(path, result)
                });
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(SetImage(path, result)) = cx.try_pop() {
                if path != self.path {
                    return;
                }

                self.state = State::Loaded;
                if let Some(image) = result {
                    // TODO(opt): we converted to RGBA8 since this is the only format common
                    // to both the image and wgpu crates. It may not be optimal however.
                    // It also assumes that the image colour space is sRGB.
                    let size = image.dimensions();

                    let draw = cx.draw_shared();
                    match draw.image_alloc(size) {
                        Ok(handle) => {
                            draw.image_upload(&handle, &image, kas::draw::ImageFormat::Rgba8);
                            self.raw.set(cx, handle);
                        }
                        Err(err) => {
                            warn_about_error("Failed to allocate image", &err);
                        }
                    }
                } else {
                    self.raw.clear(cx);
                }
            }
        }
    }
}
