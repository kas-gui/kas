// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme-applied Text element

use crate::Layout;
use crate::cast::{Cast, CastFloat};
use crate::geom::{Rect, Vec2};
use crate::layout::{AlignHints, AxisInfo, SizeRules, Stretch};
use crate::text::fonts::FontSelector;
use crate::text::format::FontToken;
use crate::text::*;
use crate::theme::{DrawCx, SizeCx, TextClass};
use std::num::NonZeroUsize;

/// A [`TextDisplay`] plus configuration and state tracking
#[derive(Clone, Debug)]
pub struct ConfiguredDisplay {
    font: FontSelector,
    dpem: f32,
    class: TextClass,
    wrap: bool,
    /// Alignment (`horiz`, `vert`)
    ///
    /// By default, horizontal alignment is left or right depending on the
    /// text direction (see [`Self::direction`]), and vertical alignment
    /// is to the top.
    align: (Align, Align),
    direction: Direction,
    status: Status,

    rect: Rect,
    display: TextDisplay,
}

impl Layout for ConfiguredDisplay {
    fn rect(&self) -> Rect {
        self.rect
    }

    /// The display should be prepared before calling this method, otherwise the
    /// result will have zero size.
    fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
        let rules = if axis.is_horizontal() {
            if self.wrap() {
                let (min, ideal) = cx.wrapped_line_len(self.class(), self.font_size());
                let bound: i32 = self
                    .measure_width(ideal.cast())
                    .map(|b| b.cast_ceil())
                    .unwrap_or_default();
                SizeRules::new(bound.min(min), bound.min(ideal), Stretch::Filler)
            } else {
                let bound: i32 = self
                    .measure_width(f32::INFINITY)
                    .map(|b| b.cast_ceil())
                    .unwrap_or_default();
                SizeRules::new(bound, bound, Stretch::Filler)
            }
        } else {
            let wrap_width = self
                .wrap()
                .then(|| axis.other().map(|w| w.cast()))
                .flatten()
                .unwrap_or(f32::INFINITY);
            let bound: i32 = self
                .measure_height(wrap_width, None)
                .map(|b| b.cast_ceil())
                .unwrap_or_default();
            SizeRules::new(bound, bound, Stretch::Filler)
        };

        rules.with_margins(cx.text_margins().extract(axis))
    }

    /// Uses default alignment where alignment is not provided
    fn set_rect(&mut self, _: &mut SizeCx, rect: Rect, hints: AlignHints) {
        self.set_align(hints.complete_default().into());
        if rect.size != self.rect.size {
            if rect.size.0 != self.rect.size.0 {
                self.set_max_status(Status::LevelRuns);
            } else {
                self.set_max_status(Status::Wrapped);
            }
        }
        self.rect = rect;
        self.rewrap();
    }

    /// Text color and decorations are not present here; derivative types will
    /// likely need their own implementation of this method.
    fn draw(&self, mut draw: DrawCx) {
        if let Ok(display) = self.display() {
            let rect = self.rect();
            draw.text(rect.pos, rect, display, &[]);
        }
    }
}

impl ConfiguredDisplay {
    /// Construct a new instance
    #[inline]
    pub fn new(class: TextClass, wrap: bool) -> Self {
        ConfiguredDisplay {
            font: FontSelector::default(),
            dpem: 16.0,
            class,
            wrap,
            align: Default::default(),
            direction: Direction::default(),
            status: Status::New,
            rect: Rect::default(),
            display: Default::default(),
        }
    }

    /// Set text class (inline)
    ///
    /// `TextClass::Edit(false)` has special handling: line wrapping is disabled
    /// and the width of self is set to that of the text.
    #[inline]
    pub fn with_class(mut self, class: TextClass) -> Self {
        self.class = class;
        self
    }

    /// Set the font and font size (dpem) according to configuration
    ///
    /// Font selection depends on the [`TextClass`], [theme configuration] and
    /// the loaded [fonts][crate::text::fonts]. Font size depends on the
    /// [`TextClass`], [theme configuration] and scale factor.
    ///
    /// Alternatively, one may call [`Self::set_font`] and
    /// [`Self::set_font_size`] or use the default values (without respecting
    /// [theme configuration]).
    ///
    /// [theme configuration]: crate::config::ThemeConfig
    pub fn configure(&mut self, cx: &mut SizeCx) {
        let font = cx.font(self.class);
        let dpem = cx.dpem(self.class);
        if font != self.font {
            self.font = font;
            self.dpem = dpem;
            self.set_max_status(Status::New);
        } else if dpem != self.dpem {
            self.dpem = dpem;
            self.set_max_status(Status::ResizeLevelRuns);
        }
    }

    /// Force full repreparation of text
    ///
    /// This may be required after calling [`Self::text_mut`].
    #[inline]
    pub fn require_reprepare(&mut self) {
        self.set_max_status(Status::New);
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
    /// `TextClass::Edit(false)` has special handling: line wrapping is disabled
    /// and the width of self is set to that of the text.
    #[inline]
    pub fn set_class(&mut self, class: TextClass) {
        self.class = class;
    }

    /// Get whether long lines are automatically wrapped
    #[inline]
    pub fn wrap(&self) -> bool {
        self.wrap
    }

    /// Set whether long lines are automatically wrapped
    #[inline]
    pub fn set_wrap(&mut self, wrap: bool) {
        self.wrap = wrap;
    }

    /// Get the font selector
    #[inline]
    pub fn font(&self) -> FontSelector {
        self.font
    }

    /// Set the font selector
    ///
    /// Typically, [`Self::configure`] is called to set the font selector from
    /// the [`TextClass`] and configuration. This method sets the font selector
    /// directly.
    ///
    /// Note that effect tokens may further affect the font selector.
    ///
    /// It is necessary to [`prepare`][Self::prepare] the text after calling this.
    #[inline]
    pub fn set_font(&mut self, font: FontSelector) {
        if font != self.font {
            self.font = font;
            self.set_max_status(Status::New);
        }
    }

    /// Get the font size (pixels)
    #[inline]
    pub fn font_size(&self) -> f32 {
        self.dpem
    }

    /// Set the font size (pixels)
    ///
    /// Typically, [`Self::configure`] is called to set the font size from
    /// the [`TextClass`] and configuration. This method sets the font size
    /// directly.
    ///
    /// Note that effect tokens may further affect the font size.
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
            self.set_max_status(Status::New);
        }
    }

    /// Get text (horizontal, vertical) alignment
    #[inline]
    pub fn align(&self) -> (Align, Align) {
        self.align
    }

    /// Set text alignment
    ///
    /// When vertical alignment is [`Align::Default`], [`Self::prepare`] will
    /// set the vertical size of this [`Layout`] to that of the text.
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

    /// Get the base directionality of the text, if prepared
    pub fn text_is_rtl(&self) -> Option<bool> {
        match self.line_is_rtl(0) {
            Ok(None) => Some(self.direction == Direction::Rtl),
            Ok(Some(is_rtl)) => Some(is_rtl),
            Err(NotReady) => None,
        }
    }

    /// Get the status
    #[inline]
    pub(crate) fn status(&self) -> Status {
        self.status
    }

    /// Check whether the status is at least `status`
    #[inline]
    pub fn check_status(&self, status: Status) -> Result<(), NotReady> {
        if self.status >= status { Ok(()) } else { Err(NotReady) }
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
    pub fn set_max_status(&mut self, status: Status) {
        self.status = self.status.min(status);
    }

    /// Set status
    #[inline]
    pub(crate) fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    /// Prepare runs, given `text` and `font_tokens`
    pub(crate) fn prepare_runs(
        &mut self,
        text: &str,
        font_tokens: impl Iterator<Item = FontToken>,
    ) {
        let direction = self.direction();
        match self.status() {
            Status::New => self
                .unchecked_display_mut()
                .prepare_runs(text, direction, font_tokens)
                .expect("no suitable font found"),
            Status::ResizeLevelRuns => self.unchecked_display_mut().resize_runs(text, font_tokens),
            _ => return,
        }

        self.set_status(Status::LevelRuns);
    }

    /// Re-wrap
    ///
    /// This is a partial form of re-preparation
    pub(crate) fn rewrap(&mut self) {
        if self.status() < Status::LevelRuns {
            return;
        }
        let align = self.align();

        if self.status() == Status::LevelRuns {
            let align_width = self.rect.size.0.cast();
            let wrap_width = if !self.wrap() { f32::INFINITY } else { align_width };
            self.unchecked_display_mut()
                .prepare_lines(wrap_width, align_width, align.0);
        }

        if self.status() <= Status::Wrapped {
            let h = self.rect.size.1.cast();
            self.unchecked_display_mut().vertically_align(h, align.1);
        }

        self.set_status(Status::Ready);
    }

    /// Read the [`TextDisplay`], without checking status
    #[inline]
    pub fn unchecked_display(&self) -> &TextDisplay {
        &self.display
    }

    /// Write to the [`TextDisplay`], without checking status
    #[inline]
    pub fn unchecked_display_mut(&mut self) -> &mut TextDisplay {
        &mut self.display
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

    /// Offset prepared content to avoid left-overhangs
    ///
    /// This might be called after [`Self::prepare`] to ensure content does not
    /// overhang to the left (i.e. that the x-component of the first [`Vec2`]
    /// returned by [`Self::bounding_box`] is not negative).
    ///
    /// This is a special utility intended for content which may be scrolled
    /// using the size reported by [`Self::bounding_box`]. Note that while
    /// vertical alignment is untouched by this method, text is never aligned
    /// above the top (the first y-component is never negative).
    pub fn ensure_no_left_overhang(&mut self) {
        if let Ok((tl, _)) = self.bounding_box()
            && tl.0 < 0.0
        {
            self.display.apply_offset(kas_text::Vec2(-tl.0, 0.0));
        }
    }

    /// Get the size of the required bounding box
    ///
    /// This is the position of the upper-left and lower-right corners of a
    /// bounding box on content.
    /// Alignment and size do affect the result.
    #[inline]
    pub fn bounding_box(&self) -> Result<(Vec2, Vec2), NotReady> {
        let (tl, br) = self.wrapped_display()?.bounding_box();
        Ok((tl.into(), br.into()))
    }

    /// Get the number of lines (after wrapping)
    ///
    /// See [`TextDisplay::num_lines`].
    #[inline]
    pub fn num_lines(&self) -> Result<usize, NotReady> {
        Ok(self.wrapped_display()?.num_lines())
    }

    /// Get line properties
    #[inline]
    pub fn get_line(&self, index: usize) -> Result<Option<&Line>, NotReady> {
        Ok(self.wrapped_display()?.get_line(index))
    }

    /// Iterate over line properties
    ///
    /// [Requires status][Self#status-of-preparation]: lines have been wrapped.
    #[inline]
    pub fn lines(&self) -> Result<impl Iterator<Item = &Line>, NotReady> {
        Ok(self.wrapped_display()?.lines())
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
        Ok(self.display()?.text_index_nearest(pos.into()))
    }

    /// Find the text index nearest horizontal-coordinate `x` on `line`
    ///
    /// See [`TextDisplay::line_index_nearest`].
    #[inline]
    pub fn line_index_nearest(&self, line: usize, x: f32) -> Result<Option<usize>, NotReady> {
        Ok(self.wrapped_display()?.line_index_nearest(line, x))
    }

    /// Measure required width, up to some `max_width`
    ///
    /// This method allows calculation of the width requirement of a text object
    /// without full wrapping and glyph placement. Whenever the requirement
    /// exceeds `max_width`, the algorithm stops early, returning `max_width`.
    ///
    /// The return value is unaffected by alignment and wrap configuration.
    pub fn measure_width(&self, max_width: f32) -> Result<f32, NotReady> {
        if self.status >= Status::LevelRuns {
            Ok(self.display.measure_width(max_width))
        } else {
            Err(NotReady)
        }
    }

    /// Measure required vertical height, wrapping as configured
    ///
    /// Stops after `max_lines`, if provided.
    pub fn measure_height(
        &self,
        wrap_width: f32,
        max_lines: Option<NonZeroUsize>,
    ) -> Result<f32, NotReady> {
        if self.status >= Status::LevelRuns {
            Ok(self.display.measure_height(wrap_width, max_lines))
        } else {
            Err(NotReady)
        }
    }

    /// Find the starting position (top-left) of the glyph at the given index
    ///
    /// See [`TextDisplay::text_glyph_pos`].
    pub fn text_glyph_pos(&self, index: usize) -> Result<MarkerPosIter, NotReady> {
        Ok(self.display()?.text_glyph_pos(index))
    }
}
