// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Image resource management

use super::DrawSharedImpl;
use image::RgbaImage;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Identifier for an image allocation
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageId(NonZeroU32);

impl ImageId {
    /// Construct a new identifier from `u32` value not equal to 0
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[inline]
    pub const fn try_new(n: u32) -> Option<Self> {
        // We can't use ? or .map in a const fn so do it the tedious way:
        if let Some(nz) = NonZeroU32::new(n) {
            Some(ImageId(nz))
        } else {
            None
        }
    }
}

/// Image formats available for upload
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ImageFormat {
    /// 8-bit unsigned RGBA values (4 bytes per pixel)
    Rgba8,
}

/// Image loading errors
#[derive(Error, Debug)]
pub enum ImageError {
    #[error("IO error")]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error("failed to allocate texture space for image")]
    Allocation,
}

pub struct Images {
    paths: HashMap<PathBuf, (ImageId, u32)>,
    images: HashMap<ImageId, RgbaImage>,
}

impl Images {
    /// Construct
    pub fn new() -> Self {
        Images {
            paths: HashMap::new(),
            images: HashMap::new(),
        }
    }

    /// Load an image from the file-system
    ///
    /// This deduplicates multiple loads of the same path, instead incrementing
    /// a reference count.
    pub fn load_path<DS: DrawSharedImpl>(
        &mut self,
        draw: &mut DS,
        path: &Path,
    ) -> Result<ImageId, ImageError> {
        if let Some((id, ref mut count)) = self.paths.get_mut(path) {
            *count += 1;
            return Ok(*id);
        }

        let image = image::io::Reader::open(path)?
            .with_guessed_format()?
            .decode()?;
        // TODO(opt): we convert to RGBA8 since this is the only format common
        // to both the image crate and WGPU. It may not be optimal however.
        // It also assumes that the image colour space is sRGB.
        let image = image.into_rgba8();
        let size = image.dimensions();

        let id = draw.image_alloc(size)?;
        draw.image_upload(id, &image, ImageFormat::Rgba8);
        self.images.insert(id, image);
        self.paths.insert(path.to_owned(), (id, 1));

        Ok(id)
    }

    /// Remove a loaded image, by path
    ///
    /// This reduces the reference count and frees if zero.
    pub fn remove_path<DS: DrawSharedImpl>(&mut self, draw: &mut DS, path: &Path) {
        let mut opt_id = None;
        self.paths.retain(|p, (id, _)| {
            if p == path {
                opt_id = Some(*id);
                false
            } else {
                true
            }
        });

        if let Some(id) = opt_id {
            self.images.remove(&id);
            draw.image_free(id);
        }
    }

    /// Remove a loaded image, by id
    ///
    /// This reduces the reference count and frees if zero.
    /// (It also removes images not created through [`Images::load_path`].)
    pub fn remove_id<DS: DrawSharedImpl>(&mut self, draw: &mut DS, id: ImageId) {
        // We don't have a map from id to path, hence have to iterate. We can
        // however do a fast check that id is used.
        if !self.images.contains_key(&id) {
            return;
        }

        let mut ref_count = 0;
        self.paths.retain(|_, obj| {
            if obj.0 == id {
                obj.1 -= 1;
                ref_count = obj.1;
                ref_count != 0
            } else {
                true
            }
        });

        if ref_count == 0 {
            self.images.remove(&id);
            draw.image_free(id);
        }
    }
}
