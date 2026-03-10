// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme-applied Text element

use super::TextClass;
#[allow(unused)] use super::{DrawCx, SizeCx};
use crate::Layout;
use crate::cast::Cast;
#[allow(unused)] use crate::event::ConfigCx;
use crate::geom::{Rect, Vec2};
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::text::ConfiguredDisplay;
use crate::text::format::{Colors, Decoration, EditableText, FormattableText};
use crate::text::*;
use kas_macros::autoimpl;
use std::num::NonZeroUsize;

/// Text type-setting object (theme aware)
///
/// This struct contains:
/// -   A [`FormattableText`]
/// -   A [`TextDisplay`]
/// -   A [`FontSelector`]
/// -   Type-setting configuration. Values have reasonable defaults:
///     -   The font is derived from the [`TextClass`] by [`Self::configure`],
///         otherwise using [`FontSelector::default()`].
///     -   The font size is derived from the [`TextClass`] by
///         [`Self::configure`], otherwise using a default size of 16px.
///     -   Default text direction and alignment is inferred from the text.
///
/// This struct tracks the [`TextDisplay`]'s
/// [state of preparation][TextDisplay#status-of-preparation] and will perform
/// steps as required. Typical usage of this struct is as follows:
/// -   Construct with some text and [`TextClass`]
/// -   Configure by calling [`Self::configure`]
/// -   Size and draw using [`Layout`] methods
#[derive(Clone, Debug)]
#[autoimpl(Deref, DerefMut using self.inner)]
pub struct Text<T: FormattableText> {
    inner: ConfiguredDisplay,
    text: T,
}

/// Implement [`Layout`], using default alignment where alignment is not provided
impl<T: FormattableText> Layout for Text<T> {
    #[inline]
    fn rect(&self) -> Rect {
        self.inner.rect()
    }

    #[inline]
    fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
        self.prepare_runs();
        self.inner.size_rules(cx, axis)
    }

    #[inline]
    fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
        self.inner.set_rect(cx, rect, hints);
    }

    #[inline]
    fn draw(&self, mut draw: DrawCx) {
        draw.text(self.rect(), self);
    }
}

impl<T: FormattableText> Text<T> {
    /// Construct from a text model
    ///
    /// This struct must be made ready for usage by calling [`Text::prepare`].
    #[inline]
    pub fn new(text: T, class: TextClass, wrap: bool) -> Self {
        Text {
            inner: ConfiguredDisplay::new(class, wrap),
            text,
        }
    }

    /// Set text class (inline)
    ///
    /// `TextClass::Edit(false)` has special handling: line wrapping is disabled
    /// and the width of self is set to that of the text.
    #[inline]
    pub fn with_class(mut self, class: TextClass) -> Self {
        self.set_class(class);
        self
    }

    /// Access the formattable text object
    #[inline]
    pub fn text(&self) -> &T {
        &self.text
    }

    /// Access the formattable text object mutably
    ///
    /// If the text is changed, one **must** call [`Self::require_reprepare`]
    /// after this method then [`Text::prepare`].
    #[inline]
    pub fn text_mut(&mut self) -> &mut T {
        &mut self.text
    }

    /// Deconstruct, taking the embedded text
    #[inline]
    pub fn take_text(self) -> T {
        self.text
    }

    /// Set the text
    ///
    /// Returns `true` when new `text` contents do not match old contents. In
    /// this case the new `text` is assigned, but the caller must also call
    /// [`Text::prepare`] afterwards.
    pub fn set_text(&mut self, text: T) -> bool {
        if self.text == text {
            return false; // no change
        }

        self.text = text;
        self.set_max_status(Status::New);
        true
    }

    /// Length of text
    ///
    /// This is a shortcut to `self.as_str().len()`.
    ///
    /// It is valid to reference text within the range `0..text_len()`,
    /// even if not all text within this range will be displayed (due to runs).
    #[inline]
    pub fn str_len(&self) -> usize {
        self.as_str().len()
    }

    /// Access whole text as contiguous `str`
    ///
    /// It is valid to reference text within the range `0..text_len()`,
    /// even if not all text within this range will be displayed (due to runs).
    #[inline]
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    /// Get the base directionality of the text
    ///
    /// This does not require that the text is prepared.
    pub fn text_is_rtl(&self) -> bool {
        let cached_is_rtl = self.inner.text_is_rtl();
        #[cfg(not(debug_assertions))]
        if let Some(cached) = cached_is_rtl {
            return cached;
        }

        let is_rtl = self
            .unchecked_display()
            .text_is_rtl(self.as_str(), self.direction());
        if let Some(cached) = cached_is_rtl {
            debug_assert_eq!(cached, is_rtl);
        }
        is_rtl
    }

    /// Return the sequence of color effect tokens
    ///
    /// This forwards to [`FormattableText::color_tokens`].
    #[inline]
    pub fn color_tokens(&self) -> &[(u32, Colors)] {
        self.text.color_tokens()
    }

    /// Return optional sequences of decoration tokens
    ///
    /// This forwards to [`FormattableText::decorations`].
    #[inline]
    pub fn decorations(&self) -> &[(u32, Decoration)] {
        self.text.decorations()
    }

    #[inline]
    fn prepare_runs(&mut self) {
        if self.status() < Status::LevelRuns {
            let (dpem, font) = (self.font_size(), self.font());
            self.inner
                .prepare_runs(self.text.as_str(), self.text.font_tokens(dpem, font));
        }
    }

    /// Measure required width, up to some `max_width`
    ///
    /// This method partially prepares the [`TextDisplay`] as required.
    ///
    /// This method allows calculation of the width requirement of a text object
    /// without full wrapping and glyph placement. Whenever the requirement
    /// exceeds `max_width`, the algorithm stops early, returning `max_width`.
    ///
    /// The return value is unaffected by alignment and wrap configuration.
    pub fn measure_width(&mut self, max_width: f32) -> f32 {
        self.prepare_runs();
        self.unchecked_display().measure_width(max_width)
    }

    /// Measure required vertical height, wrapping as configured
    ///
    /// Stops after `max_lines`, if provided.
    ///
    /// May partially prepare the text for display, but does not otherwise
    /// modify `self`.
    pub fn measure_height(&mut self, wrap_width: f32, max_lines: Option<NonZeroUsize>) -> f32 {
        self.prepare_runs();
        self.unchecked_display()
            .measure_height(wrap_width, max_lines)
    }

    /// Prepare text for display, as necessary
    ///
    /// [`Self::set_rect`] must be called before this method.
    ///
    /// Does all preparation steps necessary in order to display or query the
    /// layout of this text. Text is aligned within the set [`Rect`].
    ///
    /// Returns `true` on success when some action is performed, `false`
    /// when the text is already prepared.
    pub fn prepare(&mut self) -> bool {
        if self.is_prepared() {
            return false;
        }

        self.prepare_runs();
        debug_assert!(self.status() >= Status::LevelRuns);
        self.inner.rewrap();
        true
    }

    /// Re-prepare, requesting a redraw or resize as required
    ///
    /// The text is prepared and a redraw is requested. If the allocated size is
    /// too small, a resize is requested.
    ///
    /// This is typically called after updating a `Text` object in a widget.
    pub fn reprepare_action(&mut self, cx: &mut ConfigCx) {
        if self.prepare() {
            let (tl, br) = self.unchecked_display().bounding_box();
            let bounds: Vec2 = self.rect().size.cast();
            if tl.0 < 0.0 || tl.1 < 0.0 || br.0 > bounds.0 || br.1 > bounds.1 {
                cx.resize();
            }
        }
        cx.redraw();
    }
}

/// Text editing operations
impl<T: EditableText> Text<T> {
    /// Insert a `text` at the given position
    ///
    /// This may be used to edit the raw text instead of replacing it.
    /// One must call [`Text::prepare`] afterwards.
    ///
    /// Currently this is not significantly more efficient than
    /// [`Text::set_text`]. This may change in the future (TODO).
    #[inline]
    pub fn insert_str(&mut self, index: usize, text: &str) {
        self.text.insert_str(index, text);
        self.set_max_status(Status::New);
    }

    /// Replace a section of text
    ///
    /// This may be used to edit the raw text instead of replacing it.
    /// One must call [`Text::prepare`] afterwards.
    ///
    /// One may simulate an unbounded range by via `start..usize::MAX`.
    ///
    /// Currently this is not significantly more efficient than
    /// [`Text::set_text`]. This may change in the future (TODO).
    #[inline]
    pub fn replace_range(&mut self, range: std::ops::Range<usize>, replace_with: &str) {
        self.text.replace_range(range, replace_with);
        self.set_max_status(Status::New);
    }

    /// Replace the whole text
    ///
    /// Returns `true` when new `text` contents do not match old contents. In
    /// this case the new `text` is assigned, but the caller must also call
    /// [`Text::prepare`] afterwards.
    #[inline]
    pub fn set_str(&mut self, text: &str) -> bool {
        if self.text.as_str() == text {
            return false; // no change
        }

        self.text.set_str(text);
        self.set_max_status(Status::New);
        true
    }
}
