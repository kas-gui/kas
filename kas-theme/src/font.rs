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
use std::sync::Once;
// use wgpu_glyph::rusttype::FontCollection;

use kas::draw::{DrawTextShared, FontArc, FontId};

#[cfg(feature = "font-kit")]
use std::{fs::File, io::Read, sync::Arc};

#[cfg(feature = "font-kit")]
fn load_font() -> FontArc {
    let handle = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap();

    let (bytes, index) = match handle {
        Handle::Path { path, font_index } => {
            let mut bytes = vec![];
            File::open(path).unwrap().read_to_end(&mut bytes).unwrap();
            (bytes, font_index)
        }
        Handle::Memory { bytes, font_index } => {
            let bytes = Arc::try_unwrap(bytes).unwrap();
            (bytes, font_index)
        }
    };

    assert!(index == 0, "Font collections not yet supported");
    FontArc::try_from_vec(bytes).unwrap()
}

#[cfg(feature = "font-kit")]
lazy_static! {
    static ref FONT: FontArc = load_font();
}

#[cfg(not(feature = "font-kit"))]
const BYTES: &'static [u8] = include_bytes!("/usr/share/fonts/dejavu/DejaVuSerif.ttf");

#[cfg(not(feature = "font-kit"))]
lazy_static! {
    static ref FONT: FontArc = FontArc::try_from_slice(BYTES).unwrap();
}

/// Load fonts
pub(crate) fn load_fonts<D: DrawTextShared>(draw: &mut D) -> FontId {
    static LOAD_FONTS: Once = Once::new();
    LOAD_FONTS.call_once(|| {
        let font_id = draw.load_font(FONT.clone());
        debug_assert_eq!(font_id, FontId::default());
    });
    FontId::default()
}
