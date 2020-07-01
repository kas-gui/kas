// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Abstractions over `kas-text`

use kas::geom::{Coord, Size, Vec2};
use kas::{Align, TkAction};
pub use kas_text::{
    fonts, Font, FontId, FontScale, PreparedPart, RichText, SectionGlyph, TextPart,
};

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

    /// Layout text
    ///
    /// The given bounds are used to influence line-wrapping (if enabled).
    /// `f32::INFINITY` may be used where no bounds are required.
    ///
    /// The `scale` is used to set the base scale: rich text may adjust this.
    pub fn prepare(&mut self, bounds: Vec2, scale: FontScale) {
        self.0.set_bounds(bounds.into());
        self.0.set_base_scale(scale);
        self.0.prepare();
    }

    /// Set the text
    pub fn set_text(&mut self, text: RichText) -> TkAction {
        if self.0.set_text(text) {
            // Layout must be re-calculated which currently requires resizing
            TkAction::Resize
        } else {
            TkAction::None
        }
    }

    /// Adjust alignment
    ///
    /// This may be called before or after `prepare` and has immediate effect.
    pub fn set_alignment(&mut self, horiz: Align, vert: Align) {
        self.0.set_alignment(horiz, vert);
    }

    /// Enable or disable line-wrapping
    ///
    /// This does not have immediate effect: one must call `prepare` afterwards.
    pub fn set_line_wrap(&mut self, line_wrap: bool) {
        self.0.set_line_wrap(line_wrap);
    }

    /// Set size bounds
    ///
    /// This does not recalculate the layout. If the bounds are too small, text
    /// will be cropped.
    pub fn set_size(&mut self, size: Size) {
        self.0.set_bounds(size.into());
    }

    /// Get size bounds
    ///
    /// Returns the value last given via `prepare` or `set_size`.
    pub fn bounds(&self) -> Vec2 {
        self.0.bounds().into()
    }

    pub fn num_parts(&self) -> usize {
        self.0.num_parts()
    }

    pub fn positioned_glyphs(&self, pos: Vec2) -> Vec<SectionGlyph> {
        self.0.positioned_glyphs(pos.into())
    }

    pub fn required_size(&self) -> Vec2 {
        self.0.required_size().into()
    }

    /// Find the starting position (top-left) of the glyph at the given index
    ///
    /// May panic on invalid byte index.
    ///
    /// This method is only partially compatible with mult-line text.
    /// Ideally an external line-breaker should be used.
    pub fn text_glyph_pos(&self, pos: Coord, index: usize) -> Vec2 {
        Vec2::from(pos) + Vec2::from(self.0.text_glyph_pos(index))
    }

    /// Find the text index for the glyph nearest the given `coord`, relative to `pos`
    ///
    /// This includes the index immediately after the last glyph, thus
    /// `result ≤ text.len()`.
    ///
    /// This method is only partially compatible with mult-line text.
    /// Ideally an external line-breaker should be used.
    pub fn text_index_nearest(&self, pos: Coord, coord: Coord) -> usize {
        self.0.text_index_nearest((coord - pos).into())
    }
}
