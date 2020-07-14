// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Abstractions over `kas-text`

use kas::geom::{Coord, Vec2};
use kas::TkAction;
pub use kas_text::*;

#[doc(no_inline)]
pub use rich::Text as RichText;

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
    /// To adjust, use [`Text::update_env`].
    ///
    /// This struct must be made ready for use before
    /// To do so, call [`PreparedText::prepare`].
    pub fn new(text: rich::Text) -> Self {
        Self::new_with_env(Environment::new(), text)
    }

    /// New multi-line text
    ///
    /// This differs from [`PreparedText::new`] only in that it enables line wrapping.
    pub fn new_wrap(text: rich::Text) -> Self {
        Self::new_with_env(Environment::new_wrap(), text)
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

    /// Length of raw text
    ///
    /// It is valid to reference text within the range `0..raw_text_len()`,
    /// even if not all text within this range will be displayed (due to runs).
    pub fn raw_text_len(&self) -> usize {
        self.0.raw_text_len()
    }

    /// Layout text
    ///
    /// The given bounds are used to influence line-wrapping (if enabled).
    /// [`Vec2::INFINITY`] may be used where no bounds are required.
    ///
    /// The `scale` is used to set the base scale: rich text may adjust this.
    pub fn prepare(&mut self, bounds: Vec2, scale: FontScale) {
        self.0.update_env(|env| {
            env.set_bounds(bounds.into());
            env.set_font_scale(scale);
        });
        self.0.prepare();
    }

    /// Set the text
    ///
    /// Returns [`TkAction::Resize`] when it is necessary to call [`PreparedText::prepare`].
    pub fn set_text<T: Into<RichText>>(&mut self, text: T) -> TkAction {
        match self.0.set_text(text.into()) {
            false => TkAction::None,
            true => TkAction::Resize, // set_size calls prepare
        }
    }

    /// Read the environment
    ///
    /// Returns [`TkAction::Resize`] when it is necessary to call [`PreparedText::prepare`].
    pub fn env(&self) -> &Environment {
        self.0.env()
    }

    /// Update the environment
    ///
    /// This calls [`PreparedText::prepare`] internally.
    pub fn update_env<F: FnOnce(&mut UpdateEnv)>(&mut self, f: F) {
        self.0.update_env(f);
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
