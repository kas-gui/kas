// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text functionality
//!
//! Most of this module is simply a re-export of the [KAS Text] API, hence the
//! lower level of integration than other parts of the library.
//!
//! [`Text`] objects *must* be configured and prepared before usage, otherwise
//! they may appear empty. Call [`ConfigCx::text_config`] from
//! [`Events::configure`] and [`ConfigCx::text_set_size`] from
//! [`Layout::set_rect`] to set text position and prepare.
//! If text is adjusted, one may use e.g. [`TextApi::prepare`] to update.
//!
//! [KAS Text]: https://github.com/kas-gui/kas-text/

#[allow(unused)] use kas::{event::ConfigCx, Layout};
use kas_text::fonts::{FontId, InvalidFontId};
use kas_text::format::{EditableText, FormattableText};

pub use kas_text::*;

mod selection;
pub use selection::{SelectionAction, SelectionHelper};

mod string;
pub use string::AccessString;

/// Text type-setting object (high-level API, KAS-specific)
///
/// This struct contains:
/// -   A [`FormattableText`]
/// -   A [`TextDisplay`]
/// -   Type-setting configuration. Values have reasonable defaults:
///     -   The default font will be the first loaded font: see [fonts].
///     -   The default font size is 16px (the web default).
///     -   Default text direction and alignment is inferred from the text.
///     -   Line-wrapping requires a call to [`TextApi::set_wrap_width`].
///     -   The bounds used for alignment [must be set][TextApi::set_bounds].
///
/// This struct tracks the [`TextDisplay`]'s
/// [state of preparation][TextDisplay#status-of-preparation] and will perform
/// steps as required.
///
/// Most Functionality is implemented via the [`TextApi`] and [`TextApiExt`]
/// traits.
#[derive(Clone, Debug, Default)]
pub struct Text<T: FormattableText + ?Sized> {
    /// Bounds to use for alignment
    bounds: Vec2,
    font_id: FontId,
    dpem: f32,
    wrap_width: f32,
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

impl<T: FormattableText> Text<T> {
    /// Construct from a text model
    ///
    /// This struct must be made ready for usage by calling [`Text::prepare`].
    #[inline]
    pub fn new(text: T) -> Self {
        Text {
            bounds: Vec2::INFINITY,
            font_id: FontId::default(),
            dpem: 16.0,
            wrap_width: f32::INFINITY,
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
        /* TODO: enable if we have a way of testing equality (a hash?)
        if self.text == text {
            return; // no change
        }
         */

        self.text = text;
        self.set_max_status(Status::Configured);
    }
}

impl<T: FormattableText + ?Sized> Text<T> {
    /// Adjust status to indicate a required action
    ///
    /// This is used to notify that some step of preparation may need to be
    /// repeated. The internally-tracked status is set to the minimum of
    /// `status` and its previous value.
    #[inline]
    fn set_max_status(&mut self, status: Status) {
        self.status = self.status.min(status);
    }

    #[inline]
    fn prepare_runs(&mut self) -> Result<(), NotReady> {
        match self.status {
            Status::New => return Err(NotReady),
            Status::Configured => self
                .display
                .prepare_runs(&self.text, self.direction, self.font_id, self.dpem)
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
}

impl<T: FormattableText + ?Sized> TextApi for Text<T> {
    #[inline]
    fn check_status(&self, status: Status) -> Result<(), NotReady> {
        if self.status >= status {
            Ok(())
        } else {
            Err(NotReady)
        }
    }

    #[inline]
    fn unchecked_display(&self) -> &TextDisplay {
        &self.display
    }

    #[inline]
    fn as_str(&self) -> &str {
        self.text.as_str()
    }

    #[inline]
    fn clone_string(&self) -> String {
        self.text.as_str().to_string()
    }

    #[inline]
    fn get_font(&self) -> FontId {
        self.font_id
    }

    #[inline]
    fn set_font(&mut self, font_id: FontId) {
        if font_id != self.font_id {
            self.font_id = font_id;
            self.set_max_status(Status::Configured);
        }
    }

    #[inline]
    fn get_font_size(&self) -> f32 {
        self.dpem
    }

    #[inline]
    fn set_font_size(&mut self, dpem: f32) {
        if dpem != self.dpem {
            self.dpem = dpem;
            self.set_max_status(Status::ResizeLevelRuns);
        }
    }

    #[inline]
    fn get_direction(&self) -> Direction {
        self.direction
    }

    #[inline]
    fn set_direction(&mut self, direction: Direction) {
        if direction != self.direction {
            self.direction = direction;
            self.set_max_status(Status::Configured);
        }
    }

    #[inline]
    fn get_wrap_width(&self) -> f32 {
        self.wrap_width
    }

    #[inline]
    fn set_wrap_width(&mut self, wrap_width: f32) {
        assert!(self.wrap_width >= 0.0);
        if wrap_width != self.wrap_width {
            self.wrap_width = wrap_width;
            self.set_max_status(Status::LevelRuns);
        }
    }

    #[inline]
    fn get_align(&self) -> (Align, Align) {
        self.align
    }

    #[inline]
    fn set_align(&mut self, align: (Align, Align)) {
        if align != self.align {
            if align.0 == self.align.0 {
                self.set_max_status(Status::Wrapped);
            } else {
                self.set_max_status(Status::LevelRuns);
            }
            self.align = align;
        }
    }

    #[inline]
    fn get_bounds(&self) -> Vec2 {
        self.bounds
    }

    #[inline]
    fn set_bounds(&mut self, bounds: Vec2) {
        debug_assert!(bounds.is_finite());
        if bounds != self.bounds {
            if bounds.0 != self.bounds.0 {
                self.set_max_status(Status::LevelRuns);
            } else {
                self.set_max_status(Status::Wrapped);
            }
            self.bounds = bounds;
        }
    }

    fn set_font_properties(
        &mut self,
        direction: Direction,
        font_id: FontId,
        dpem: f32,
        wrap_width: f32,
    ) {
        self.set_font(font_id);
        self.set_font_size(dpem);
        self.set_direction(direction);
        self.set_wrap_width(wrap_width);
    }

    #[inline]
    fn text_is_rtl(&self) -> bool {
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

    #[inline]
    fn configure(&mut self) -> Result<(), InvalidFontId> {
        // Validate default_font_id
        let _ = fonts::library().first_face_for(self.font_id)?;

        self.status = self.status.max(Status::Configured);
        Ok(())
    }

    fn line_height(&self) -> Result<f32, NotReady> {
        self.check_status(Status::Configured)?;

        fonts::library()
            .get_first_face(self.get_font())
            .map(|face| face.height(self.get_font_size()))
            .map_err(|_| {
                debug_assert!(false, "font_id should be validated by configure");
                NotReady
            })
    }

    fn measure_width(&mut self, max_width: f32) -> Result<f32, NotReady> {
        self.prepare_runs()?;

        Ok(self.display.measure_width(max_width))
    }

    fn measure_height(&mut self) -> Result<f32, NotReady> {
        if self.status >= Status::Wrapped {
            let (tl, br) = self.display.bounding_box();
            return Ok(br.1 - tl.1);
        }

        self.prepare_runs()?;
        Ok(self.display.measure_height(self.wrap_width))
    }

    #[inline]
    fn prepare(&mut self) -> Result<bool, NotReady> {
        self.prepare_runs()?;
        debug_assert!(self.status >= Status::LevelRuns);

        if self.status == Status::LevelRuns {
            self.display
                .prepare_lines(self.wrap_width, self.bounds.0, self.align.0);
        }

        let overflow = if self.status <= Status::Wrapped {
            let bound = self.display.vertically_align(self.bounds.1, self.align.1);
            !(bound.0 <= self.bounds.0 && bound.1 <= self.bounds.1)
        } else {
            false
        };

        self.status = Status::Ready;
        Ok(overflow)
    }

    #[inline]
    fn effect_tokens(&self) -> &[Effect<()>] {
        self.text.effect_tokens()
    }
}

impl<T: EditableText + ?Sized> EditableTextApi for Text<T> {
    #[inline]
    fn insert_char(&mut self, index: usize, c: char) {
        self.text.insert_char(index, c);
        self.set_max_status(Status::Configured);
    }

    #[inline]
    fn replace_range(&mut self, range: std::ops::Range<usize>, replace_with: &str) {
        self.text.replace_range(range, replace_with);
        self.set_max_status(Status::Configured);
    }

    #[inline]
    fn set_string(&mut self, string: String) {
        self.text.set_string(string);
        self.set_max_status(Status::Configured);
    }

    #[inline]
    fn swap_string(&mut self, string: &mut String) {
        self.text.swap_string(string);
        self.set_max_status(Status::Configured);
    }
}
