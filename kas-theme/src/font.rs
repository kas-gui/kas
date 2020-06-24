// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Font management
//!
//! Optionally, this uses font-kit to find a suitable font. Since this is a
//! large dependency, an alternative is provided.

use kas::draw::DrawTextShared;
use kas::text::FontId;
use std::sync::Once;

#[cfg(feature = "font-kit")]
fn load_fonts_impl<D: DrawTextShared>(draw: &mut D) -> FontId {
    use font_kit::{
        family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource,
    };
    use std::{fs::File, io::Read, sync::Arc};

    let handle = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap();

    let (bytes, font_index) = match handle {
        Handle::Path { path, font_index } => {
            let mut bytes = vec![];
            File::open(path).unwrap().read_to_end(&mut bytes).unwrap();
            (bytes, font_index)
        }
        Handle::Memory { bytes, font_index } => match Arc::try_unwrap(bytes) {
            Ok(v) => (v, font_index),
            Err(a) => ((*a).clone(), font_index),
        },
    };

    draw.load_font_vec(bytes, font_index)
}

#[cfg(not(feature = "font-kit"))]
fn load_fonts_impl<D: DrawTextShared>(draw: &mut D) -> FontId {
    const BYTES: &'static [u8] = include_bytes!("/usr/share/fonts/dejavu/DejaVuSerif.ttf");
    draw.load_font_static_ref(BYTES, 0)
}

/// Load fonts
pub(crate) fn load_fonts<D: DrawTextShared>(draw: &mut D) -> FontId {
    static LOAD_FONTS: Once = Once::new();
    LOAD_FONTS.call_once(|| {
        let font_id = load_fonts_impl(draw);
        debug_assert_eq!(font_id, FontId::default());
    });
    FontId::default()
}
