// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Abstractions over `kas-text`

use kas::geom::{Coord, Quad, Vec2};
use kas::TkAction;
pub use kas_text::*;

#[doc(no_inline)]
pub use rich::Text as RichText;

/// Prepare action needed after text object updates
#[must_use]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum PrepareAction {
    /// No action needed
    None,
    /// Prepare must be called
    Prepare,
}

impl std::ops::Add for PrepareAction {
    type Output = Self;

    #[inline]
    fn add(self, rhs: PrepareAction) -> Self {
        self.max(rhs)
    }
}

impl std::ops::AddAssign for PrepareAction {
    #[inline]
    fn add_assign(&mut self, rhs: PrepareAction) {
        *self = (*self).max(rhs);
    }
}

/// Text, prepared for display in a given enviroment
///
/// Text is laid out for display in a box with the given size.
///
/// The type can be default-constructed with no text.
#[derive(Clone, Debug, Default)]
pub struct PreparedText(prepared::Text);

impl PreparedText {
    /// New single-line text
    ///
    /// This method assumes single-line mode and default alignment.
    /// To adjust, use [`PreparedText::update_env`].
    ///
    /// This struct must be made ready for use before
    /// To do so, call [`PreparedText::prepare`].
    pub fn new(text: rich::Text) -> Self {
        // Note: wrap is on by default for Environment, but here it makes more
        // sense to turn it off in the default constructor.
        let mut env = Environment::new();
        env.wrap = false;
        Self::new_with_env(env, text)
    }

    /// New multi-line text
    ///
    /// This differs from [`PreparedText::new`] only in that it enables line wrapping.
    pub fn new_wrap(text: rich::Text) -> Self {
        Self::new_with_env(Environment::new(), text)
    }

    /// New multi-line text
    ///
    /// This differs from [`PreparedText::new`] in that it allows explicit
    /// setting of [`Environment`] parameters.
    pub fn new_with_env(env: Environment, text: rich::Text) -> Self {
        PreparedText(prepared::Text::new(env, text))
    }

    /// Reconstruct the [`RichText`] defining this `PreparedText`
    pub fn clone_text(&self) -> RichText {
        self.0.clone_text()
    }

    /// Clone the raw string
    pub fn clone_string(&self) -> String {
        self.0.text().to_string()
    }

    /// Length of raw text
    ///
    /// It is valid to reference text within the range `0..text_len()`,
    /// even if not all text within this range will be displayed (due to runs).
    pub fn text_len(&self) -> usize {
        self.0.text_len()
    }

    /// Access to the raw text
    ///
    /// This is the contiguous raw text without formatting information.
    pub fn text(&self) -> &str {
        self.0.text()
    }

    /// Insert a char at the given position
    ///
    /// This may be used to edit the raw text instead of replacing it.
    /// One must call [`PreparedText::prepare`] afterwards.
    ///
    /// TODO: document how this affects formatting.
    ///
    /// Currently this is not significantly more efficent than
    /// [`PreparedText::set_text`]. This may change in the future (TODO).
    ///
    /// Returns [`PrepareAction::Prepare`]: i.e. this does not cause resizing.
    pub fn insert_char(&mut self, index: usize, c: char) -> PrepareAction {
        self.0.insert_char(index, c);
        PrepareAction::Prepare
    }

    /// Replace a section of text
    ///
    /// This may be used to edit the raw text instead of replacing it.
    /// One must call [`PreparedText::prepare`] afterwards.
    ///
    /// TODO: document how this affects formatting.
    ///
    /// Currently this is not significantly more efficent than
    /// [`PreparedText::set_text`]. This may change in the future (TODO).
    ///
    /// Returns [`PrepareAction::Prepare`]: i.e. this does not cause resizing.
    pub fn replace_range<R>(&mut self, range: R, replace_with: &str) -> PrepareAction
    where
        R: std::ops::RangeBounds<usize>,
    {
        self.0.replace_range(range, replace_with);
        PrepareAction::Prepare
    }

    /// Swap the raw text with a `String`
    ///
    /// This may be used to edit the raw text instead of replacing it.
    /// One must call [`PreparedText::prepare`] afterwards.
    ///
    /// TODO: document how this affects formatting.
    ///
    /// Currently this is not significantly more efficent than
    /// [`PreparedText::set_text`]. This may change in the future (TODO).
    ///
    /// Returns [`PrepareAction::Prepare`]: i.e. this does not cause resizing.
    pub fn swap_string(&mut self, string: &mut String) -> PrepareAction {
        self.0.swap_string(string);
        PrepareAction::Prepare
    }

    /// Set the text
    ///
    /// See also [`PreparedText::set_and_prepare`].
    pub fn set_text<T: Into<RichText>>(&mut self, text: T) -> PrepareAction {
        match self.0.set_text(text.into()) {
            false => PrepareAction::None,
            true => PrepareAction::Prepare, // set_size calls prepare
        }
    }

    /// Set the text
    ///
    /// This calls [`PreparedText::prepare`] internally, then returns
    /// [`TkAction::Redraw`]. (This does not force a resize.)
    pub fn set_and_prepare<T: Into<RichText>>(&mut self, text: T) -> TkAction {
        if self.0.set_text(text.into()) {
            self.0.prepare();
            TkAction::Redraw
        } else {
            TkAction::None
        }
    }

    /// Read the environment
    pub fn env(&self) -> &Environment {
        self.0.env()
    }

    /// Update the environment and prepare for display
    ///
    /// This calls [`PreparedText::prepare`] to prepare text for display.
    pub fn update_env<F: FnOnce(&mut UpdateEnv)>(&mut self, f: F) {
        self.0.update_env(f);
    }

    /// Prepare text for display
    pub fn prepare(&mut self) {
        self.0.prepare();
    }

    pub fn positioned_glyphs<G, F: Fn(&str, FontId, PxScale, Glyph) -> G>(&self, f: F) -> Vec<G> {
        // TODO: Should we cache this result somewhere? Unfortunately we still
        // don't have the type G here, and the caller (draw_text.rs) does not
        // have storage directly associated with this PreparedText.
        self.0.positioned_glyphs(f)
    }

    pub fn required_size(&self) -> Vec2 {
        self.0.required_size().into()
    }

    /// Get the number of lines
    pub fn num_lines(&self) -> usize {
        self.0.num_lines()
    }

    /// Find the line containing text `index`
    ///
    /// Returns the line number and the text-range of the line.
    ///
    /// Returns `None` in case `index` does not line on or at the end of a line
    /// (which means either that `index` is beyond the end of the text or that
    /// `index` is within a mult-byte line break).
    pub fn find_line(&self, index: usize) -> Option<(usize, std::ops::Range<usize>)> {
        self.0.find_line(index)
    }

    /// Get the range of a line, by line number
    pub fn line_range(&self, line: usize) -> Option<std::ops::Range<usize>> {
        self.0.line_range(line)
    }

    /// Find the starting position (top-left) of the glyph at the given index
    ///
    /// Returns `Some(pos, ascent, descent)` on success, where `pos.1 - ascent`
    /// and `pos.1 - descent` are the top and bottom of the glyph position.
    ///
    /// Note that this only searches *visible* text sections for a valid index.
    /// In case the `index` is not within a slice of visible text, this returns
    /// `None`. So long as `index` is within a visible slice (or at its end),
    /// it does not need to be on a valid code-point.
    ///
    /// Note: if the text's bounding rect does not start at the origin, then
    /// the coordinates of the top-left corner should be added to this result.
    pub fn text_glyph_pos(
        &self,
        pos: Coord,
        index: usize,
    ) -> impl DoubleEndedIterator<Item = (Vec2, f32, f32)> {
        let x = Vec2::from(pos);
        self.0
            .text_glyph_pos(index)
            .map(move |item| (x + Vec2::from(item.pos), item.ascent, item.descent))
    }

    /// Find the starting position (top-left) of the glyph at the given index
    ///
    /// This differs from [`PreparedText::text_glyph_pos`] in that it does not
    /// offset the result by a coordinate `pos`.
    pub fn text_glyph_rel_pos(
        &self,
        index: usize,
    ) -> impl DoubleEndedIterator<Item = (Vec2, f32, f32)> {
        self.0
            .text_glyph_pos(index)
            .map(|item| (Vec2::from(item.pos), item.ascent, item.descent))
    }

    /// Find the text index for the glyph nearest the given `coord`, relative to `pos`
    ///
    /// This includes the index immediately after the last glyph, thus
    /// `result ≤ text.len()`.
    pub fn text_index_nearest(&self, pos: Coord, coord: Coord) -> usize {
        self.0.text_index_nearest((coord - pos).into())
    }

    /// Find the text index nearest horizontal-coordinate `x` on `line`
    ///
    /// Unlike [`PreparedText::text_index_nearest`], this does not offset `x`.
    /// It also allows the line to be specified explicitly.
    pub fn line_index_nearest(&self, line: usize, x: f32) -> Option<usize> {
        self.0.line_index_nearest(line, x)
    }

    /// Yield a sequence of rectangles to highlight a given range, by lines
    ///
    /// Rectangles span to end and beginning of lines when wrapping lines.
    ///
    /// This locates the ends of a range as with [`PreparedText::text_glyph_pos`], but
    /// yields a separate rect for each "run" within this range (where "run" is
    /// is a line or part of a line). Rects are represented by the top-left
    /// vertex and the bottom-right vertex.
    pub fn highlight_lines<R: Into<std::ops::Range<usize>>>(
        &self,
        pos: Coord,
        range: R,
    ) -> Vec<Quad> {
        let pos = Vec2::from(pos);
        self.0
            .highlight_lines(range)
            .iter()
            .map(|(p1, p2)| Quad::with_coords(pos + Vec2::from(*p1), pos + Vec2::from(*p2)))
            .collect()
    }

    /// Yield a sequence of rectangles to highlight a given range, by runs
    ///
    /// Rectangles tightly fit each "run" (piece) of text highlighted.
    ///
    /// This locates the ends of a range as with [`PreparedText::text_glyph_pos`], but
    /// yields a separate rect for each "run" within this range (where "run" is
    /// is a line or part of a line). Rects are represented by the top-left
    /// vertex and the bottom-right vertex.
    pub fn highlight_runs<R: Into<std::ops::Range<usize>>>(
        &self,
        pos: Coord,
        range: R,
    ) -> Vec<Quad> {
        let pos = Vec2::from(pos);
        self.0
            .highlight_runs(range)
            .iter()
            .map(|(p1, p2)| Quad::with_coords(pos + Vec2::from(*p1), pos + Vec2::from(*p2)))
            .collect()
    }
}
