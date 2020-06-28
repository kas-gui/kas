// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Abstractions over `kas-text`

use kas::geom::{Size, Vec2};
use kas::{Align, TkAction};
pub use kas_text::{fonts, Font, FontId, FontScale, PreparedPart, RichText, TextPart};

/// Text, prepared for display in a given enviroment
///
/// Text is laid out for display in a box with the given size.
///
/// The type can be default-constructed with no text.
#[derive(Clone, Debug, Default)]
pub struct PreparedText(kas_text::PreparedText);

impl PreparedText {
    /// Construct from a text model
    ///
    /// This method assumes default alignment. To adjust, use [`PreparedText::set_alignment`].
    ///
    /// This struct must be made ready for use before
    /// To do so, call [`PreparedText::set_environment`].
    pub fn new(text: RichText, line_wrap: bool) -> PreparedText {
        PreparedText(kas_text::PreparedText::new(text, line_wrap))
    }

    /// Reconstruct the [`RichText`] defining this `PreparedText`
    pub fn clone_text(&self) -> RichText {
        self.0.clone_text()
    }

    /// Index at end of text
    pub fn total_len(&self) -> usize {
        self.0.total_len()
    }

    /// Set the text
    pub fn set_text(&mut self, text: RichText) -> TkAction {
        self.0.set_text(text);
        if self.0.require_font() {
            // TODO: this should be Resize
            TkAction::Reconfigure
        } else {
            TkAction::None
        }
    }

    /// Adjust alignment
    pub fn set_alignment(&mut self, horiz: Align, vert: Align) {
        self.0.set_alignment(horiz, vert)
    }

    /// Enable or disable line-wrapping
    pub fn set_line_wrap(&mut self, line_wrap: bool) {
        self.0.set_line_wrap(line_wrap);
    }

    /// Set fonts
    pub fn set_font(&mut self, font_id: FontId, scale: FontScale) {
        self.0.set_font(font_id, scale);
    }

    /// Set size bounds
    pub fn set_size(&mut self, size: Size) {
        self.0.set_size(size.into())
    }

    pub fn align_horiz(&self) -> Align {
        self.0.align_horiz()
    }
    pub fn align_vert(&self) -> Align {
        self.0.align_vert()
    }
    pub fn line_wrap(&self) -> bool {
        self.0.line_wrap()
    }

    pub fn size(&self) -> Vec2 {
        self.0.size().into()
    }

    pub fn parts<'a>(&'a self) -> impl Iterator<Item = &'a PreparedPart> {
        self.0.parts()
    }
}
