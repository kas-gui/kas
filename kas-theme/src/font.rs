// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Font management
//!
//! Optionally, this uses font-kit to find a suitable font. Since this is a
//! large dependency, an alternative is provided.

#[cfg(feature = "font-kit")]
use font_kit::{
    family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource,
};

use lazy_static::lazy_static;
use rusttype::Font;
// use wgpu_glyph::rusttype::FontCollection;

#[cfg(feature = "font-kit")]
use std::{fs::File, io::Read, sync::Arc};

#[cfg(feature = "font-kit")]
struct FontCollectionBytes {
    bytes: Vec<u8>,
    index: u32,
}

#[cfg(feature = "font-kit")]
impl FontCollectionBytes {
    fn load() -> Self {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::SansSerif], &Properties::new())
            .unwrap();
        match handle {
            Handle::Path { path, font_index } => {
                let mut bytes = vec![];
                File::open(path).unwrap().read_to_end(&mut bytes).unwrap();
                FontCollectionBytes {
                    bytes,
                    index: font_index,
                }
            }
            Handle::Memory { bytes, font_index } => {
                let bytes = Arc::try_unwrap(bytes).unwrap();
                FontCollectionBytes {
                    bytes,
                    index: font_index,
                }
            }
        }
    }
    fn font<'a>(&'a self) -> Font<'a> {
        // FontCollection is in next version of rusttype
        assert!(self.index == 0, "Font collections not yet supported");
        Font::from_bytes(&self.bytes).unwrap()
    }
}

#[cfg(feature = "font-kit")]
lazy_static! {
    static ref FCB: FontCollectionBytes = FontCollectionBytes::load();
    static ref FONT: Font<'static> = FCB.font();
}

#[cfg(not(feature = "font-kit"))]
const BYTES: &'static [u8] = include_bytes!("/usr/share/fonts/dejavu/DejaVuSerif.ttf");

#[cfg(not(feature = "font-kit"))]
lazy_static! {
    static ref FONT: Font<'static> = Font::from_bytes(BYTES).unwrap();
}

/// Get access to the font
///
/// TODO: this function is a placeholder until proper font management is
/// integrated.
pub fn get_font() -> Font<'static> {
    FONT.clone()
}
