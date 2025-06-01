// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme-applied Text element

use super::TextClass;
#[allow(unused)] use super::{DrawCx, SizeCx};
use crate::cast::Cast;
#[allow(unused)] use crate::event::ConfigCx;
use crate::geom::{Rect, Size};
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::text::fonts::{FontSelector, InvalidFontId};
use crate::text::format::{EditableText, FormattableText};
use crate::text::*;
use crate::{Action, Layout};

/// Text type-setting object (theme aware)
///
/// This struct contains:
/// -   A [`FormattableText`]
/// -   A [`TextDisplay`]
/// -   A [`FontSelector`]
/// -   Type-setting configuration. Values have reasonable defaults:
///     -   The font is derived from the [`TextClass`] by
///         [`ConfigCx::text_configure`]. Otherwise, the default font will be
///         the first loaded font: see [`crate::text::fonts`].
///     -   The font size is derived from the [`TextClass`] by
///         [`ConfigCx::text_configure`]. Otherwise, the default font size is
///         16px (the web default).
///     -   Default text direction and alignment is inferred from the text.
///
/// This struct tracks the [`TextDisplay`]'s
/// [state of preparation][TextDisplay#status-of-preparation] and will perform
/// steps as required. Normal usage of this struct is as follows:
/// -   Configure by calling [`ConfigCx::text_configure`]
/// -   (Optionally) check size requirements by calling [`SizeCx::text_rules`]
/// -   Set the size and prepare by calling [`Self::set_rect`]
/// -   Draw by calling [`DrawCx::text`] (and/or other text methods)
#[derive(Clone, Debug)]
pub struct Text<T: FormattableText> {
    rect: Rect,
    font: FontSelector,
    dpem: f32,
    class: TextClass,
    /// Alignment (`horiz`, `vert`)
    ///
    /// By default, horizontal alignment is left or right depending on the
    /// text direction (see [`Self::direction`]), and vertical alignment
    /// is to the top.
    align: (Align, Align),
    direction: Direction,
    status: Status,

    display: TextDisplay,
    text: T,
}

impl<T: Default + FormattableText> Default for Text<T> {
    fn default() -> Self {
        Self::new(T::default(), TextClass::Label(true))
    }
}

/// Implement [`Layout`], using default alignment where alignment is not provided
impl<T: FormattableText> Layout for Text<T> {
    fn rect(&self) -> Rect {
        self.rect
    }

    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        sizer.text_rules(self, axis)
    }

    fn set_rect(&mut self, _: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        self.set_align(hints.complete_default().into());
        if rect.size != self.rect.size {
            if rect.size.0 != self.rect.size.0 {
                self.set_max_status(Status::LevelRuns);
            } else {
                self.set_max_status(Status::Wrapped);
            }
        }
        self.rect = rect;
        self.prepare().expect("not configured");
    }

    fn draw(&self, mut draw: DrawCx) {
        draw.text(self.rect, self);
    }
}

impl<T: FormattableText> Text<T> {
    /// Construct from a text model
    ///
    /// This struct must be made ready for usage by calling [`Text::prepare`].
    #[inline]
    pub fn new(text: T, class: TextClass) -> Self {
        Text {
            rect: Rect::default(),
            font: FontSelector::default(),
            dpem: 16.0,
            class,
            align: Default::default(),
            direction: Direction::default(),
            status: Status::New,
            text,
            display: Default::default(),
        }
    }

    /// Replace the [`TextDisplay`]
    ///
    /// This may be used with [`Self::new`] to reconstruct an object which was
    /// disolved [`into_parts`][Self::into_parts].
    #[inline]
    pub fn with_display(mut self, display: TextDisplay) -> Self {
        self.display = display;
        self
    }

    /// Decompose into parts
    #[inline]
    pub fn into_parts(self) -> (TextDisplay, T) {
        (self.display, self.text)
    }

    /// Set text class (inline)
    ///
    /// Default: `TextClass::Label(true)`
    #[inline]
    pub fn with_class(mut self, class: TextClass) -> Self {
        self.class = class;
        self
    }

    /// Clone the formatted text
    pub fn clone_text(&self) -> T
    where
        T: Clone,
    {
        self.text.clone()
    }

    /// Extract text object, discarding the rest
    #[inline]
    pub fn take_text(self) -> T {
        self.text
    }

    /// Access the formattable text object
    #[inline]
    pub fn text(&self) -> &T {
        &self.text
    }

    /// Set the text
    ///
    /// One must call [`Text::prepare`] afterwards and may wish to inspect its
    /// return value to check the size allocation meets requirements.
    pub fn set_text(&mut self, text: T) {
        if self.text == text {
            return; // no change
        }

        self.text = text;
        self.set_max_status(Status::Configured);
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

    /// Clone the unformatted text as a `String`
    #[inline]
    pub fn clone_string(&self) -> String {
        self.text.as_str().to_string()
    }

    /// Get text class
    #[inline]
    pub fn class(&self) -> TextClass {
        self.class
    }

    /// Set text class
    ///
    /// This controls line-wrapping, font and font size selection.
    ///
    /// Default: `TextClass::Label(true)`
    #[inline]
    pub fn set_class(&mut self, class: TextClass) {
        self.class = class;
    }

    /// Get the default font
    #[inline]
    pub fn font(&self) -> FontSelector {
        self.font
    }

    /// Set the default [`FontSelector`]
    ///
    /// This is derived from the [`TextClass`] by [`ConfigCx::text_configure`].
    ///
    /// This `font` is used by all unformatted texts and by any formatted
    /// texts which don't immediately set formatting.
    ///
    /// It is necessary to [`prepare`][Self::prepare] the text after calling this.
    #[inline]
    pub fn set_font(&mut self, font: FontSelector) {
        if font != self.font {
            self.font = font;
            self.set_max_status(Status::Configured);
        }
    }

    /// Get the default font size (pixels)
    #[inline]
    pub fn font_size(&self) -> f32 {
        self.dpem
    }

    /// Set the default font size (pixels)
    ///
    /// This is derived from the [`TextClass`] by [`ConfigCx::text_configure`].
    ///
    /// This is a scaling factor used to convert font sizes, with units
    /// `pixels/Em`. Equivalently, this is the line-height in pixels.
    /// See [`crate::text::fonts`] documentation.
    ///
    /// To calculate this from text size in Points, use `dpem = dpp * pt_size`
    /// where the dots-per-point is usually `dpp = scale_factor * 96.0 / 72.0`
    /// on PC platforms, or `dpp = 1` on MacOS (or 2 for retina displays).
    ///
    /// It is necessary to [`prepare`][Self::prepare] the text after calling this.
    #[inline]
    pub fn set_font_size(&mut self, dpem: f32) {
        if dpem != self.dpem {
            self.dpem = dpem;
            self.set_max_status(Status::ResizeLevelRuns);
        }
    }

    /// Set font size
    ///
    /// This is an alternative to [`Text::set_font_size`]. It is assumed
    /// that 72 Points = 1 Inch and the base screen resolution is 96 DPI.
    /// (Note: MacOS uses a different definition where 1 Point = 1 Pixel.)
    #[inline]
    pub fn set_font_size_pt(&mut self, pt_size: f32, scale_factor: f32) {
        self.set_font_size(pt_size * scale_factor * (96.0 / 72.0));
    }

    /// Get the base text direction
    #[inline]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Set the base text direction
    ///
    /// It is necessary to [`prepare`][Self::prepare] the text after calling this.
    #[inline]
    pub fn set_direction(&mut self, direction: Direction) {
        if direction != self.direction {
            self.direction = direction;
            self.set_max_status(Status::Configured);
        }
    }

    /// Get text (horizontal, vertical) alignment
    #[inline]
    pub fn align(&self) -> (Align, Align) {
        self.align
    }

    /// Set text alignment
    ///
    /// It is necessary to [`prepare`][Self::prepare] the text after calling this.
    #[inline]
    pub fn set_align(&mut self, align: (Align, Align)) {
        if align != self.align {
            if align.0 == self.align.0 {
                self.set_max_status(Status::Wrapped);
            } else {
                self.set_max_status(Status::LevelRuns);
            }
            self.align = align;
        }
    }

    /// Get text size
    #[inline]
    pub fn size(&self) -> Size {
        self.rect.size
    }

    /// Get the base directionality of the text
    ///
    /// This does not require that the text is prepared.
    pub fn text_is_rtl(&self) -> bool {
        let cached_is_rtl = match self.line_is_rtl(0) {
            Ok(None) => Some(self.direction == Direction::Rtl),
            Ok(Some(is_rtl)) => Some(is_rtl),
            Err(NotReady) => None,
        };
        #[cfg(not(debug_assertions))]
        if let Some(cached) = cached_is_rtl {
            return cached;
        }

        let is_rtl = self.display.text_is_rtl(self.as_str(), self.direction);
        if let Some(cached) = cached_is_rtl {
            debug_assert_eq!(cached, is_rtl);
        }
        is_rtl
    }

    /// Get the sequence of effect tokens
    ///
    /// This method has some limitations: (1) it may only return a reference to
    /// an existing sequence, (2) effect tokens cannot be generated dependent
    /// on input state, and (3) it does not incorporate color information. For
    /// most uses it should still be sufficient, but for other cases it may be
    /// preferable not to use this method (use a dummy implementation returning
    /// `&[]` and use inherent methods on the text object via [`Text::text`]).
    #[inline]
    pub fn effect_tokens(&self) -> &[Effect<()>] {
        self.text.effect_tokens()
    }
}

/// Type-setting operations and status
impl<T: FormattableText> Text<T> {
    /// Check whether the status is at least `status`
    #[inline]
    pub fn check_status(&self, status: Status) -> Result<(), NotReady> {
        if self.status >= status {
            Ok(())
        } else {
            Err(NotReady)
        }
    }

    /// Check whether the text is fully prepared and ready for usage
    #[inline]
    pub fn is_prepared(&self) -> bool {
        self.status == Status::Ready
    }

    /// Adjust status to indicate a required action
    ///
    /// This is used to notify that some step of preparation may need to be
    /// repeated. The internally-tracked status is set to the minimum of
    /// `status` and its previous value.
    #[inline]
    fn set_max_status(&mut self, status: Status) {
        self.status = self.status.min(status);
    }

    /// Read the [`TextDisplay`], without checking status
    #[inline]
    pub fn unchecked_display(&self) -> &TextDisplay {
        &self.display
    }

    /// Read the [`TextDisplay`], if fully prepared
    #[inline]
    pub fn display(&self) -> Result<&TextDisplay, NotReady> {
        self.check_status(Status::Ready)?;
        Ok(self.unchecked_display())
    }

    /// Read the [`TextDisplay`], if at least wrapped
    #[inline]
    pub fn wrapped_display(&self) -> Result<&TextDisplay, NotReady> {
        self.check_status(Status::Wrapped)?;
        Ok(self.unchecked_display())
    }

    /// Configure text
    ///
    /// Text objects must be configured before use.
    #[inline]
    pub fn configure(&mut self) -> Result<(), InvalidFontId> {

        self.status = self.status.max(Status::Configured);
        Ok(())
    }

    fn prepare_runs(&mut self) -> Result<(), NotReady> {
        match self.status {
            Status::New => return Err(NotReady),
            Status::Configured => self
                .display
                .prepare_runs(&self.text, self.direction, self.font, self.dpem)
                .map_err(|_| {
                    debug_assert!(false, "font_id should be validated by configure");
                    NotReady
                })?,
            Status::ResizeLevelRuns => self.display.resize_runs(&self.text, self.dpem),
            _ => (),
        }

        self.status = Status::LevelRuns;
        Ok(())
    }

    /// Measure required width, up to some `max_width`
    ///
    /// [`configure`][Self::configure] must be called before this method.
    ///
    /// This method partially prepares the [`TextDisplay`] as required.
    ///
    /// This method allows calculation of the width requirement of a text object
    /// without full wrapping and glyph placement. Whenever the requirement
    /// exceeds `max_width`, the algorithm stops early, returning `max_width`.
    ///
    /// The return value is unaffected by alignment and wrap configuration.
    pub fn measure_width(&mut self, max_width: f32) -> Result<f32, NotReady> {
        self.prepare_runs()?;

        Ok(self.display.measure_width(max_width))
    }

    /// Measure required vertical height
    ///
    /// [`configure`][Self::configure] must be called before this method.
    /// May partially prepare the text for display, but does not otherwise
    /// modify `self`.
    pub fn measure_height(&mut self, wrap_width: f32) -> Result<f32, NotReady> {
        if self.status >= Status::Wrapped {
            let (tl, br) = self.display.bounding_box();
            return Ok(br.1 - tl.1);
        }

        self.prepare_runs()?;
        Ok(self.display.measure_height(wrap_width))
    }

    /// Prepare text for display, as necessary
    ///
    /// [`Self::configure`] and [`Self::set_rect`] must be called before this
    /// method.
    ///
    /// Does all preparation steps necessary in order to display or query the
    /// layout of this text. Text is aligned within the set [`Rect`].
    ///
    /// Returns `Ok(true)` on success when some action is performed, `Ok(false)`
    /// when the text is already prepared.
    pub fn prepare(&mut self) -> Result<bool, NotReady> {
        if self.is_prepared() {
            return Ok(false);
        }

        self.prepare_runs()?;
        debug_assert!(self.status >= Status::LevelRuns);

        if self.status == Status::LevelRuns {
            let bounds: Vec2 = self.rect.size.cast();
            self.display.prepare_lines(bounds.0, bounds.0, self.align.0);
        }

        if self.status <= Status::Wrapped {
            self.display
                .vertically_align(self.rect.size.1.cast(), self.align.1);
        }

        self.status = Status::Ready;
        Ok(true)
    }

    /// Re-prepare, if previously prepared, and return an [`Action`]
    ///
    /// Wraps [`Text::prepare`], returning an appropriate [`Action`]:
    ///
    /// -   When this `Text` object was previously prepared and has sufficient
    ///     size, it is updated and [`Action::REDRAW`] is returned
    /// -   When this `Text` object was previously prepared but does not have
    ///     sufficient size, it is updated and [`Action::RESIZE`] is returned
    /// -   When this `Text` object was not previously prepared,
    ///     [`Action::empty()`] is returned without updating `self`.
    ///
    /// This is typically called after updating a `Text` object in a widget.
    pub fn reprepare_action(&mut self) -> Action {
        match self.prepare() {
            Err(NotReady) => Action::empty(),
            Ok(false) => Action::REDRAW,
            Ok(true) => {
                let (tl, br) = self.display.bounding_box();
                let bounds: Vec2 = self.rect.size.cast();
                if tl.0 < 0.0 || tl.1 < 0.0 || br.0 > bounds.0 || br.1 > bounds.1 {
                    Action::RESIZE
                } else {
                    Action::REDRAW
                }
            }
        }
    }
    /// Get the size of the required bounding box
    ///
    /// This is the position of the upper-left and lower-right corners of a
    /// bounding box on content.
    /// Alignment and size do affect the result.
    #[inline]
    pub fn bounding_box(&self) -> Result<(Vec2, Vec2), NotReady> {
        Ok(self.wrapped_display()?.bounding_box())
    }
    /// Get the number of lines (after wrapping)
    ///
    /// See [`TextDisplay::num_lines`].
    #[inline]
    pub fn num_lines(&self) -> Result<usize, NotReady> {
        Ok(self.wrapped_display()?.num_lines())
    }

    /// Find the line containing text `index`
    ///
    /// See [`TextDisplay::find_line`].
    #[inline]
    pub fn find_line(
        &self,
        index: usize,
    ) -> Result<Option<(usize, std::ops::Range<usize>)>, NotReady> {
        Ok(self.wrapped_display()?.find_line(index))
    }

    /// Get the range of a line, by line number
    ///
    /// See [`TextDisplay::line_range`].
    #[inline]
    pub fn line_range(&self, line: usize) -> Result<Option<std::ops::Range<usize>>, NotReady> {
        Ok(self.wrapped_display()?.line_range(line))
    }

    /// Get the directionality of the current line
    ///
    /// See [`TextDisplay::line_is_rtl`].
    #[inline]
    pub fn line_is_rtl(&self, line: usize) -> Result<Option<bool>, NotReady> {
        Ok(self.wrapped_display()?.line_is_rtl(line))
    }

    /// Find the text index for the glyph nearest the given `pos`
    ///
    /// See [`TextDisplay::text_index_nearest`].
    #[inline]
    pub fn text_index_nearest(&self, pos: Vec2) -> Result<usize, NotReady> {
        Ok(self.display()?.text_index_nearest(pos))
    }

    /// Find the text index nearest horizontal-coordinate `x` on `line`
    ///
    /// See [`TextDisplay::line_index_nearest`].
    #[inline]
    pub fn line_index_nearest(&self, line: usize, x: f32) -> Result<Option<usize>, NotReady> {
        Ok(self.wrapped_display()?.line_index_nearest(line, x))
    }

    /// Find the starting position (top-left) of the glyph at the given index
    ///
    /// See [`TextDisplay::text_glyph_pos`].
    pub fn text_glyph_pos(&self, index: usize) -> Result<MarkerPosIter, NotReady> {
        Ok(self.display()?.text_glyph_pos(index))
    }
}

/// Text editing operations
impl Text<String> {
    /// Insert a char at the given position
    ///
    /// This may be used to edit the raw text instead of replacing it.
    /// One must call [`Text::prepare`] afterwards.
    ///
    /// Currently this is not significantly more efficient than
    /// [`Text::set_text`]. This may change in the future (TODO).
    #[inline]
    pub fn insert_char(&mut self, index: usize, c: char) {
        self.text.insert_char(index, c);
        self.set_max_status(Status::Configured);
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
        self.set_max_status(Status::Configured);
    }

    /// Set text to a raw `String`
    ///
    /// Returns `true` when new `text` contents do not match old contents. In
    /// this case the new `text` is assigned, but the caller must also call
    /// [`Text::prepare`] afterwards.
    #[inline]
    pub fn set_string(&mut self, text: String) -> bool {
        if self.text.as_str() == text {
            return false; // no change
        }

        self.text.set_string(text);
        self.set_max_status(Status::Configured);
        true
    }

    /// Swap the raw text with a `String`
    ///
    /// This may be used to edit the raw text instead of replacing it.
    /// One must call [`Text::prepare`] afterwards.
    ///
    /// Currently this is not significantly more efficient than
    /// [`Text::set_text`]. This may change in the future (TODO).
    #[inline]
    pub fn swap_string(&mut self, string: &mut String) {
        self.text.swap_string(string);
        self.set_max_status(Status::Configured);
    }
}

/// Required functionality on [`Text`] objects for sizing by the theme
pub trait SizableText {
    /// Set font face and size
    fn set_font(&mut self, font: FontSelector, dpem: f32);

    /// Configure text
    fn configure(&mut self) -> Result<(), InvalidFontId>;

    /// Measure required width, up to some `max_width`
    fn measure_width(&mut self, max_width: f32) -> Result<f32, NotReady>;

    /// Measure required vertical height, wrapping as configured
    fn measure_height(&mut self, wrap_width: f32) -> Result<f32, NotReady>;
}

impl<T: FormattableText> SizableText for Text<T> {
    fn set_font(&mut self, font: FontSelector, dpem: f32) {
        if font != self.font {
            self.font = font;
            self.dpem = dpem;
            self.set_max_status(Status::Configured);
        } else if dpem != self.dpem {
            self.dpem = dpem;
            self.set_max_status(Status::ResizeLevelRuns);
        }
    }

    fn configure(&mut self) -> Result<(), InvalidFontId> {
        Text::configure(self)
    }

    fn measure_width(&mut self, max_width: f32) -> Result<f32, NotReady> {
        Text::measure_width(self, max_width)
    }

    fn measure_height(&mut self, wrap_width: f32) -> Result<f32, NotReady> {
        Text::measure_height(self, wrap_width)
    }
}
