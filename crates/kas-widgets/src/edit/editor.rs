// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text editor components
//!
//! The struct [`Editor`] provides a public API for text-editing actions.
//!
//! [`Component`] is a lower-level type for integrating a text editor into a
//! widget (this is used, for example, in [`EditBoxCore`].
//!
//! [`Common`] and [`Part`] are lower-level components of [`Component`]: a
//! single-paragraph editor should have one of each while a multi-paragraph
//! editor might use multiple [`Part`]s.

use super::highlight::{self, Highlighter, SchemeColors};
use super::*;
use kas::cast::Cast;
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{
    ConfigCx, ElementState, FocusSource, Ime, ImePurpose, ImeSurroundingText, Scroll,
};
use kas::geom::{Rect, Vec2};
use kas::layout::{AlignHints, AxisInfo, SizeRules};
use kas::prelude::*;
use kas::text::fonts::FontSelector;
use kas::text::{CursorRange, Direction, NotReady, Status, TextDisplay, format};
use kas::theme::{Background, DrawCx, SizeCx, TextClass};
use kas::util::UndoStack;
use kas::{Layout, autoimpl};
use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::rc::Rc;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

/// Action: text parts should have their status reset to [`Status::New`] and be re-prepared
#[must_use]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionResetStatus;

/// Result type of [`Component::handle_event`]
pub enum EventAction {
    /// Key not used, no action
    Unused,
    /// Key used, no action
    Used,
    /// Focus has been gained
    FocusGained,
    /// Focus has been lost
    FocusLost,
    /// Cursor and/or selection changed
    Cursor,
    /// Enter key in single-line editor
    Activate(Option<PhysicalKey>),
    /// Transient (uncommitted) edit by IME
    Preedit,
    /// Text was edited by key command
    Edit,
}

impl EventAction {
    /// If true, text has been edited and must be re-prepared.
    pub fn requires_repreparation(&self) -> bool {
        matches!(self, EventAction::Preedit | EventAction::Edit)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct TextIndex {
    part: u32,
    byte: u32,
}

impl TextIndex {
    fn new(part: impl Cast<u32>, byte: impl Cast<u32>) -> Self {
        TextIndex {
            part: part.cast(),
            byte: byte.cast(),
        }
    }

    fn part(&self) -> usize {
        self.part.cast()
    }

    fn byte(&self) -> usize {
        self.byte.cast()
    }
}

fn subrange_of(range: &Range<TextIndex>, part: u32) -> Range<usize> {
    debug_assert!(range.start.part == part && range.end.part == part);
    range.start.byte()..range.end.byte()
}

/// Editor state common to all parts
#[derive(Debug)]
pub struct Common {
    /// We store a copy of the widget id here, since the latter is inaccessible
    id: Id,
    colors: SchemeColors,
    font: FontSelector,
    dpem: f32,
    direction: Direction,
    wrap: bool,
    read_only: bool,
    has_key_focus: bool,
    edit_x_coord: Option<f32>,
    selection: CursorRange<TextIndex>,
    last_edit: Option<EditOp>,
    /// Stack items: (first_part_num, num_parts, Vec of saved texts from first_part_num, selection)
    undo_stack: UndoStack<(usize, usize, Vec<Rc<String>>, CursorRange<TextIndex>)>,
    current: CurrentAction,
    input_handler: TextInput,
}

impl Common {
    /// Construct a new instance
    #[inline]
    pub fn new(wrap: bool) -> Self {
        Common {
            id: Id::default(),
            colors: SchemeColors::default(),
            font: FontSelector::default(),
            dpem: 16.0,
            direction: Direction::Auto,
            wrap,
            read_only: false,
            has_key_focus: false,
            edit_x_coord: None,
            selection: CursorRange::default(),
            last_edit: Some(EditOp::Initial),
            undo_stack: UndoStack::new(),
            current: CurrentAction::None,
            input_handler: Default::default(),
        }
    }

    /// Configure `Common` data
    #[inline]
    #[must_use]
    pub fn configure(&mut self, cx: &SizeCx, id: Id) -> Option<ActionResetStatus> {
        self.id = id;
        let font = cx.font(TextClass::Editor);
        let dpem = cx.dpem(TextClass::Editor);
        if font != self.font || dpem != self.dpem {
            self.font = font;
            self.dpem = dpem;
            Some(ActionResetStatus)
        } else {
            None
        }
    }

    /// Read highlighter colors
    #[inline]
    pub fn colors(&self) -> &SchemeColors {
        &self.colors
    }

    /// Get the theme-defined background color
    #[inline]
    pub fn background_color(&self) -> Background {
        if let Some(c) = self.colors.background.as_rgba() {
            Background::Rgb(c.as_rgb())
        } else {
            Background::Default
        }
    }
}

/// A text part for usage by an editor
///
/// ### Special behaviour
///
/// The wrapping widget may (optionally) wish to implement [`Viewport`] to
/// support scrolling of text content. Since this component is not a widget it
/// cannot implement [`Viewport`] directly, but it does provide the following
/// methods: [`Self::content_size`], [`Self::draw_with_offset`].
#[autoimpl(Debug)]
pub struct Part {
    part: u32, // part index
    rect: Rect,
    status: Status,
    display: TextDisplay,
    highlight: highlight::Cache,
    text: Rc<String>,
}

/// A list of parts
#[allow(clippy::len_without_is_empty)]
pub trait PartList {
    fn len(&self) -> usize;

    fn get(&self, part: usize) -> &Part;
    fn get_mut(&mut self, part: usize) -> &mut Part;

    fn iter(&self) -> impl Iterator<Item = &Part>;
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Part>;

    /// If `true`, this list supports insertion and deletion; if `false`, the
    /// list has exactly one `Part`.
    fn variable_length(&self) -> bool;
    fn insert(&mut self, index: usize, part: Part);
    fn delete(&mut self, index: usize);
}

impl PartList for Part {
    #[inline]
    fn len(&self) -> usize {
        1
    }

    #[inline]
    fn get(&self, part: usize) -> &Part {
        assert!(part == 0, "invalid part index");
        self
    }

    #[inline]
    fn get_mut(&mut self, part: usize) -> &mut Part {
        assert!(part == 0, "invalid part index");
        self
    }

    #[inline]
    fn iter(&self) -> impl Iterator<Item = &Part> {
        std::iter::once(self)
    }

    #[inline]
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Part> {
        std::iter::once(self)
    }

    #[inline]
    fn variable_length(&self) -> bool {
        false
    }

    #[inline]
    fn insert(&mut self, _: usize, _: Part) {
        unimplemented!()
    }

    #[inline]
    fn delete(&mut self, _: usize) {
        unimplemented!()
    }
}

/// Inner editor interface
///
/// This type provides an API usable by [`EditGuard`] and (read-only) via
/// [`Deref`](std::ops::Deref) from [`EditBoxCore`] and [`EditBox`].
#[autoimpl(Debug)]
pub struct Editor {
    common: Common,
    part: Part,
    error_state: Option<Option<Cow<'static, str>>>,
}

/// Editor component
///
/// This is a component used to implement an editor widget. It is used, for
/// example, in [`EditBoxCore`].
///
/// ### Special behaviour
///
/// This component implements [`Layout`], but only requests the minimum size
/// allocation required to display its current text contents. The wrapping
/// widget may wish to reserve extra space, use a higher stretch policy and
/// potentially also set an alignment hint.
///
/// See also [`Part`] (accessible through [`Self::part`]).
#[derive(Debug)]
pub struct Component<H: Highlighter>(pub Editor, pub H);

impl<H: Highlighter> Layout for Component<H> {
    #[inline]
    fn rect(&self) -> Rect {
        self.0.part.rect
    }

    #[inline]
    fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
        self.0.part.size_rules(&self.0.common, cx, axis)
    }

    #[inline]
    fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, _: AlignHints) {
        self.0.part.set_rect(&self.0.common, cx, rect);
    }

    #[inline]
    fn draw(&self, draw: DrawCx) {
        self.0
            .part
            .draw_with_offset(draw, &self.0.common, self.rect(), Offset::ZERO);
    }
}

impl<H: Highlighter> Component<H> {
    /// Construct a new instance
    #[inline]
    pub fn new(wrap: bool) -> Self
    where
        H: Default,
    {
        let editor = Editor {
            common: Common::new(wrap),
            part: Part::default(),
            error_state: None,
        };
        Component(editor, H::default())
    }

    /// Set whether long lines are automatically wrapped
    #[inline]
    pub fn set_wrap(&mut self, wrap: bool) {
        self.0.common.wrap = wrap;
        self.0.part.status = self.0.part.status.min(Status::LevelRuns);
    }

    /// Set the base text direction
    ///
    /// If [`Direction::Auto`] or [`Direction::AutoRtl`] is used, the direction
    /// will be updated on edit to persist the last used text direction to
    /// non-directional content.
    #[inline]
    pub fn set_direction(&mut self, direction: Direction) {
        self.0.common.direction = direction;
        self.0.part.status = Status::New;
    }

    /// Replace the highlighter
    #[inline]
    pub fn with_highlighter<H2: Highlighter>(self, highlighter: H2) -> Component<H2> {
        Component(self.0, highlighter)
    }

    /// Set a new highlighter of the same type
    pub fn set_highlighter(&mut self, highlighter: H) {
        self.1 = highlighter;
        self.0.part.require_reprepare();
    }

    /// Get the background color
    ///
    /// Uses the UI theme's error color if applicable.
    pub fn background_color(&self) -> Background {
        if self.0.error_state.is_some() {
            Background::Error
        } else {
            self.0.common.background_color()
        }
    }

    /// Set the initial text (inline)
    ///
    /// This method should only be used on a new `Component`.
    #[inline]
    #[must_use]
    pub fn with_text(mut self, text: impl ToString) -> Self {
        debug_assert!(self.0.common.last_edit == Some(EditOp::Initial));

        self.0.part.text = Rc::new(text.to_string());
        let byte = if self.0.common.wrap { 0 } else { self.0.part.text.len() };
        let index = TextIndex::new(0, byte);
        self.0.common.selection.set_position(index);
        self
    }

    /// Access the text part
    #[inline]
    pub fn part(&self) -> &Part {
        &self.0.part
    }

    /// Configure component
    #[inline]
    pub fn configure(&mut self, cx: &mut ConfigCx, id: Id) {
        if let Some(ActionResetStatus) = self.0.common.configure(&cx.size_cx(), id) {
            self.0.part.require_reprepare();
        }
        if let Some(_) = self.1.configure(cx) {
            self.0.common.colors = self.1.scheme_colors();
            self.0.part.require_reprepare();
        }

        self.0.part.prepare_runs(&mut self.0.common, &mut self.1);
    }

    /// Fully prepare text for display
    ///
    /// This method performs all required steps of preparation according to the
    /// [`Status`] (which is advanced to [`Status::Ready`]).
    ///
    /// It is usually preferable to call [`Self::prepare_and_scroll`] after
    /// edits to the text to trigger any required resizing and scrolling.
    #[inline]
    pub fn prepare(&mut self) {
        if self.0.part.is_prepared() {
            return;
        }

        self.0.part.prepare_runs(&mut self.0.common, &mut self.1);
        self.0.part.prepare_wrap(&self.0.common);
    }

    /// Fully prepare text for display, ensuring the cursor is within view
    ///
    /// This method performs all required steps of preparation according to the
    /// [`Status`] (which is advanced to [`Status::Ready`]). This method should
    /// be called after changes to the text, alignment or wrap-width.
    #[inline]
    pub fn prepare_and_scroll(&mut self, cx: &mut EventCx) {
        self.0
            .common
            .prepare_and_scroll(&mut self.0.part, &mut self.1, cx);
    }

    /// Measure required vertical height, wrapping as configured
    ///
    /// Stops after `max_lines`, if provided.
    ///
    /// May partially prepare the text for display, but does not otherwise
    /// modify `self`.
    #[inline]
    pub fn measure_height(&mut self, wrap_width: f32, max_lines: Option<NonZeroUsize>) -> f32 {
        self.0.part.prepare_runs(&mut self.0.common, &mut self.1);
        self.0.part.display.measure_height(wrap_width, max_lines)
    }

    /// Implementation of [`Viewport::draw_with_offset`]
    #[inline]
    pub fn draw_with_offset(&self, draw: DrawCx, rect: Rect, offset: Offset) {
        self.0
            .part
            .draw_with_offset(draw, &self.0.common, rect, offset);
    }

    /// Handle an event
    #[inline]
    pub fn handle_event(&mut self, cx: &mut EventCx, event: Event) -> EventAction {
        let action = self.0.common.handle_event(&mut self.0.part, cx, event);
        if action.requires_repreparation() {
            self.0
                .common
                .prepare_and_scroll(&mut self.0.part, &mut self.1, cx);
        }
        action
    }

    /// Clear the error state
    #[inline]
    pub fn clear_error(&mut self) {
        self.0.error_state = None;
    }
}

impl Default for Part {
    #[inline]
    fn default() -> Self {
        Part {
            part: 0,
            rect: Rect::ZERO,
            status: Status::New,
            display: TextDisplay::default(),
            highlight: Default::default(),
            text: Default::default(),
        }
    }
}

impl Part {
    /// Get text contents
    #[inline]
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    /// Get the base directionality of the text
    ///
    /// [`Self::prepare_runs`] should be called before this method.
    #[inline]
    pub fn text_is_rtl(&self) -> bool {
        debug_assert!(self.status >= Status::ResizeLevelRuns);
        self.display.text_is_rtl()
    }

    /// Check whether the text is fully prepared and ready for usage
    #[inline]
    pub fn is_prepared(&self) -> bool {
        self.status == Status::Ready
    }

    /// Force full repreparation of text
    #[inline]
    pub fn require_reprepare(&mut self) {
        self.status = Status::New;
    }

    /// Perform run-breaking and shaping
    ///
    /// This represents a high-level step of preparation required before
    /// displaying text. This method should be called before any sizing
    /// operations. This will advance the [`Status`] to [`Status::LevelRuns`].
    /// This method must be called again after any edits to the `Part`'s text.
    #[inline]
    pub fn prepare_runs<H: Highlighter>(&mut self, common: &mut Common, highlighter: &mut H) {
        fn inner<H: Highlighter>(part: &mut Part, common: &mut Common, highlighter: &mut H) {
            part.highlight.highlight(&part.text, highlighter);

            let text = part.text.as_str();
            let font_tokens = part.highlight.font_tokens(common.dpem, common.font);
            match part.status {
                Status::New => part
                    .display
                    .prepare_runs(text, common.direction, font_tokens)
                    .expect("no suitable font found"),
                Status::ResizeLevelRuns => part.display.resize_runs(text, font_tokens),
                _ => return,
            }

            part.status = Status::LevelRuns;

            if common.direction.is_auto() {
                common.direction = if part.display.text_is_rtl() {
                    Direction::AutoRtl
                } else {
                    Direction::Auto
                };
            }
        }

        if self.status < Status::LevelRuns {
            inner(self, common, highlighter);
        }
    }

    /// Get the assigned [`Rect`]
    #[inline]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Solve size rules
    pub fn size_rules(&mut self, common: &Common, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
        let rules = if axis.is_horizontal() {
            let mut bound = 0i32;
            if common.wrap {
                let (min, ideal) = cx.wrapped_line_len(TextClass::Editor, common.dpem);
                if self.status >= Status::LevelRuns {
                    bound = self.display.measure_width(ideal.cast()).cast_ceil();
                }
                SizeRules::new(bound.min(min), bound.min(ideal), Stretch::Filler)
            } else {
                if self.status >= Status::LevelRuns {
                    bound = self.display.measure_width(f32::INFINITY).cast_ceil();
                }
                SizeRules::new(bound, bound, Stretch::Filler)
            }
        } else {
            let wrap_width = common
                .wrap
                .then(|| axis.other().map(|w| w.cast()))
                .flatten()
                .unwrap_or(f32::INFINITY);
            let mut bound = 0i32;
            if self.status >= Status::LevelRuns {
                bound = self.display.measure_height(wrap_width, None).cast_ceil();
            }
            SizeRules::new(bound, bound, Stretch::Filler)
        };

        rules.with_margins(cx.text_margins().extract(axis))
    }

    /// Set rect
    ///
    /// This `rect` is stored and available through [`Self::rect`].
    ///
    /// Changing the width requires re-wrapping lines; other changes to `rect`
    /// should be very cheap.
    ///
    /// Note that editors always use default alignment of content.
    pub fn set_rect(&mut self, common: &Common, cx: &mut SizeCx, rect: Rect) {
        if rect.size.0 != self.rect.size.0 {
            self.status = self.status.min(Status::LevelRuns);
        }
        self.rect = rect;

        self.prepare_wrap(common);
        if common.current.is_ime_enabled() {
            self.set_ime_cursor_area(common, cx);
        }
    }

    /// Directly set the position
    ///
    /// This may be called instead of [`Self::set_rect`] if only `pos` changes.
    #[inline]
    pub fn set_pos(&mut self, pos: Coord) {
        self.rect.pos = pos;
    }

    /// Perform line wrapping and alignment
    ///
    /// This represents a high-level step of preparation required before
    /// displaying text. After [run-breaking](Self::prepare_runs), this method
    /// should be called before displaying the text. This will advance
    /// the [status](ConfiguredDisplay::status) to [`Status::Ready`].
    /// This method must be called again after [`Self::prepare_runs`] and after
    /// changes to alignment or the wrap-width.
    ///
    /// Returns `true` when the size of the bounding-box changes.
    fn prepare_wrap(&mut self, common: &Common) -> bool {
        if self.status < Status::LevelRuns || self.rect.size.0 == 0 {
            return false;
        };

        let bb = self.display.bounding_box();

        if self.status == Status::LevelRuns {
            let align_width = self.rect.size.0.cast();
            let wrap_width = if !common.wrap { f32::INFINITY } else { align_width };
            self.display
                .prepare_lines(wrap_width, align_width, Align::Default);
            self.display.ensure_non_negative_alignment();
        }

        self.status = Status::Ready;
        bb != self.display.bounding_box()
    }

    /// Measure required vertical height, wrapping as configured
    ///
    /// Stops after `max_lines`, if provided.
    ///
    /// [`Self::prepare_runs`] should be called before this.
    pub fn measure_height(
        &mut self,
        wrap_width: f32,
        max_lines: Option<NonZeroUsize>,
    ) -> Result<f32, NotReady> {
        if self.status >= Status::LevelRuns {
            Ok(self.display.measure_height(wrap_width, max_lines))
        } else {
            Err(NotReady)
        }
    }

    /// Implementation of [`Viewport::content_size`]
    pub fn content_size(&self) -> Size {
        if self.status < Status::Wrapped {
            return Size::ZERO;
        }

        let (tl, br) = self.display.bounding_box();
        (Vec2::from(br) - Vec2::from(tl)).cast_ceil()
    }

    /// Implementation of [`Viewport::draw_with_offset`]
    pub fn draw_with_offset(&self, mut draw: DrawCx, common: &Common, rect: Rect, offset: Offset) {
        if !self.is_prepared() {
            return;
        }

        let pos = self.rect.pos - offset;
        let range = common.selection.to_range();
        let range = if self.part < range.start.part || self.part > range.end.part {
            0..0
        } else {
            let start = if self.part == range.start.part {
                range.start.byte
            } else {
                debug_assert!(self.part > range.start.part);
                0
            };
            let end = if self.part == range.end.part {
                range.end.byte
            } else {
                debug_assert!(self.part < range.end.part);
                self.text.len().cast()
            };
            start..end
        };

        let color_tokens = self.highlight.color_tokens();
        let default_colors = format::Colors {
            foreground: common.colors.foreground,
            background: None,
        };
        let mut buf = [(0, default_colors); 3];
        let mut vec = vec![];
        let tokens = if range.is_empty() {
            if color_tokens.is_empty() {
                &buf[..1]
            } else {
                color_tokens
            }
        } else if color_tokens.is_empty() {
            buf[1].0 = range.start;
            buf[1].1.foreground = common.colors.selection_foreground;
            buf[1].1.background = Some(common.colors.selection_background);
            buf[2].0 = range.end;
            let r0 = if range.start > 0 { 0 } else { 1 };
            &buf[r0..]
        } else {
            let set_selection_colors = |c: &mut format::Colors| {
                if c.foreground == common.colors.foreground {
                    c.foreground = common.colors.selection_foreground;
                }
                c.background = Some(common.colors.selection_background);
            };

            vec.reserve(color_tokens.len() + 2);
            let mut i = 0;
            let mut change_index = range.start;
            let mut in_selection = false;
            while i < color_tokens.len() {
                let (start, mut colors) = color_tokens[i];
                if start < change_index {
                    if in_selection {
                        set_selection_colors(&mut colors);
                    }
                } else if start == change_index {
                    in_selection = change_index == range.start;
                    if in_selection {
                        set_selection_colors(&mut colors);
                        change_index = range.end;
                    } else {
                        change_index = u32::MAX;
                    }
                } else {
                    let index = change_index;
                    let mut colors = if i > 0 {
                        color_tokens[i - 1].1
                    } else {
                        Default::default()
                    };
                    in_selection = change_index == range.start;
                    if in_selection {
                        change_index = range.end;
                        set_selection_colors(&mut colors);
                    } else {
                        change_index = u32::MAX;
                    };
                    vec.push((index, colors));
                    continue;
                }
                vec.push((start, colors));
                i += 1;
            }
            let last_colors = if i > 0 {
                color_tokens[i - 1].1
            } else {
                Default::default()
            };
            if change_index == range.start {
                let mut colors = last_colors;
                set_selection_colors(&mut colors);
                vec.push((range.start, colors));
                change_index = range.end;
            }
            if change_index == range.end {
                vec.push((range.end, last_colors));
            }
            &vec
        };
        draw.text(pos, rect, &self.display, tokens);

        let decorations = self.highlight.decorations();
        if !decorations.is_empty() {
            draw.decorate_text(pos, rect, &self.display, decorations);
        }

        if let CurrentAction::ImePreedit { edit_range, .. } = common.current.clone() {
            let tokens = [
                Default::default(),
                (edit_range.start, format::Decoration {
                    dec: format::DecorationType::Underline,
                    ..Default::default()
                }),
                (edit_range.end, Default::default()),
            ];
            let r0 = if edit_range.start > 0 { 0 } else { 1 };
            draw.decorate_text(pos, rect, &self.display, &tokens[r0..]);
        }

        if !common.read_only
            && self.part == common.selection.cursor.part
            && draw.ev_state().has_input_focus(&common.id) == Some(true)
        {
            draw.text_cursor(
                pos,
                rect,
                &self.display,
                common.selection.cursor.byte(),
                Some(common.colors.cursor),
            );
        }
    }

    /// Replace a section of text
    #[inline]
    fn replace_range(&mut self, range: Range<usize>, replace_with: &str) {
        Rc::make_mut(&mut self.text).replace_range(range, replace_with);
        self.require_reprepare();
    }

    fn trim_paste(&self, wrap: bool, text: &str) -> Range<usize> {
        let mut end = text.len();
        if !wrap {
            // We cut the content short on control characters and
            // ignore them (preventing line-breaks and ignoring any
            // actions such as recursive-paste).
            for (i, c) in text.char_indices() {
                if c < '\u{20}' || ('\u{7f}'..='\u{9f}').contains(&c) {
                    end = i;
                    break;
                }
            }
        }
        0..end
    }

    /// Clean up IME state
    ///
    /// One should also call [`EventCx::cancel_ime_focus`] unless this is
    /// implied.
    fn clear_ime(&mut self, common: &mut Common) {
        if common.current.is_ime_enabled() {
            let action = std::mem::replace(&mut common.current, CurrentAction::None);
            if let CurrentAction::ImePreedit { edit_range, .. } = action {
                common
                    .selection
                    .set_position(TextIndex::new(self.part, edit_range.start));
                self.replace_range(edit_range.cast(), "");
            }
        }
    }

    fn ime_surrounding_text(&self, common: &Common) -> Option<ImeSurroundingText> {
        const MAX_TEXT_BYTES: usize = ImeSurroundingText::MAX_TEXT_BYTES;

        let sel_range = subrange_of(&common.selection.to_range(), self.part);
        let edit_range = match common.current.clone() {
            CurrentAction::ImePreedit { edit_range, .. } => Some(edit_range.cast()),
            _ => None,
        };
        let mut range = edit_range.clone().unwrap_or(sel_range);
        let initial_range = range.clone();
        let edit_len = edit_range.clone().map(|r| r.len()).unwrap_or(0);

        if self.status >= Status::Wrapped {
            if let Some((_, line_range)) = self.display.find_line(range.start) {
                range.start = line_range.start;
            }
            if let Some((_, line_range)) = self.display.find_line(range.end) {
                range.end = line_range.end;
            }
        }

        if range.len() - edit_len > MAX_TEXT_BYTES {
            range.end = range.end.min(initial_range.end + MAX_TEXT_BYTES / 2);
            while !self.as_str().is_char_boundary(range.end) {
                range.end -= 1;
            }

            if range.len() - edit_len > MAX_TEXT_BYTES {
                range.start = range.start.max(initial_range.start - MAX_TEXT_BYTES / 2);
                while !self.as_str().is_char_boundary(range.start) {
                    range.start += 1;
                }
            }
        }

        let start = range.start;
        let mut text = String::with_capacity(range.len() - edit_len);
        if let Some(er) = edit_range {
            text.push_str(&self.as_str()[range.start..er.start]);
            text.push_str(&self.as_str()[er.end..range.end]);
        } else {
            text = self.as_str()[range].to_string();
        }

        let cursor = common.selection.cursor.byte().saturating_sub(start);
        let anchor = common.selection.anchor.byte().saturating_sub(start);
        ImeSurroundingText::new(text, cursor, anchor)
            .inspect_err(|err| {
                // TODO: use Display for err not Debug
                log::warn!("Editor::ime_surrounding_text failed: {err:?}")
            })
            .ok()
    }

    /// Call to set IME position only while IME is active
    fn set_ime_cursor_area(&self, common: &Common, cx: &mut EventState) {
        if !self.is_prepared() {
            return;
        }

        let range = match common.current.clone() {
            CurrentAction::ImeStart(_) => subrange_of(&common.selection.to_range(), self.part),
            CurrentAction::ImePreedit { edit_range, .. } => edit_range.cast(),
            _ => return,
        };

        let (m1, m2);
        if range.is_empty() {
            let mut iter = self.display.text_glyph_pos(range.start);
            m1 = iter.next();
            m2 = iter.next();
        } else {
            m1 = self.display.text_glyph_pos(range.start).next_back();
            m2 = self.display.text_glyph_pos(range.end).next();
        }

        let rect = if let Some((c1, c2)) = m1.zip(m2) {
            let left = c1.pos.0.min(c2.pos.0);
            let right = c1.pos.0.max(c2.pos.0);
            let top = (c1.pos.1 - c1.ascent).min(c2.pos.1 - c2.ascent);
            let bottom = (c1.pos.1 - c1.descent).max(c2.pos.1 - c2.ascent);
            let p1 = Vec2(left, top).cast_floor();
            let p2 = Vec2(right, bottom).cast_ceil();
            Rect::from_coords(p1, p2)
        } else if let Some(c) = m1.or(m2) {
            let p1 = Vec2(c.pos.0, c.pos.1 - c.ascent).cast_floor();
            let p2 = Vec2(c.pos.0, c.pos.1 - c.descent).cast_ceil();
            Rect::from_coords(p1, p2)
        } else {
            return;
        };

        cx.set_ime_cursor_area(&common.id, rect + Offset::conv(self.rect.pos));
    }

    /// Handle IME methods on a given `Part`.
    fn handle_ime(&mut self, common: &mut Common, cx: &mut EventCx, event: Ime) -> EventAction {
        match event {
            Ime::Enabled => EventAction::Unused,
            Ime::Disabled => {
                self.clear_ime(common);
                if !common.has_key_focus {
                    EventAction::FocusLost
                } else {
                    EventAction::Used
                }
            }
            Ime::Preedit { text, cursor } => {
                let (part, mut edit_range) = match common.current.clone() {
                    CurrentAction::ImeStart(part) if cursor.is_some() => {
                        (part, subrange_of(&common.selection.to_range(), part))
                    }
                    CurrentAction::ImeStart(_) => return EventAction::Used,
                    CurrentAction::ImePreedit { part, edit_range } => (part, edit_range.cast()),
                    _ => return EventAction::Used,
                };

                self.replace_range(edit_range.clone(), text);
                edit_range.end = edit_range.start + text.len();
                if let Some((start, end)) = cursor {
                    common.selection.anchor = TextIndex::new(part, edit_range.start + start);
                    common.selection.cursor = TextIndex::new(part, edit_range.start + end);
                } else {
                    common
                        .selection
                        .set_position(TextIndex::new(part, edit_range.start + text.len()));
                }

                common.current = CurrentAction::ImePreedit {
                    part,
                    edit_range: edit_range.cast(),
                };
                common.edit_x_coord = None;
                EventAction::Preedit
            }
            Ime::Commit { text } => {
                let (part, edit_range) = match common.current.clone() {
                    CurrentAction::ImeStart(part) => {
                        (part, subrange_of(&common.selection.to_range(), part))
                    }
                    CurrentAction::ImePreedit { part, edit_range } => (part, edit_range.cast()),
                    _ => return EventAction::Used,
                };

                self.replace_range(edit_range.clone(), text);
                common
                    .selection
                    .set_position(TextIndex::new(part, edit_range.start + text.len()));

                common.current = CurrentAction::ImePreedit {
                    part,
                    edit_range: edit_range.cast(),
                };
                common.edit_x_coord = None;
                EventAction::Edit
            }
            Ime::DeleteSurrounding {
                before_bytes,
                after_bytes,
            } => {
                let edit_range = match common.current.clone() {
                    CurrentAction::ImeStart(part) => {
                        subrange_of(&common.selection.to_range(), part)
                    }
                    CurrentAction::ImePreedit { edit_range, .. } => edit_range.cast(),
                    _ => return EventAction::Used,
                };

                if before_bytes > 0 {
                    let end = edit_range.start;
                    let start = end - before_bytes;
                    if self.as_str().is_char_boundary(start) {
                        self.replace_range(start..end, "");
                        let len = end - start;
                        let adjust = |index: &mut TextIndex| {
                            if index.byte() >= end {
                                index.byte -= u32::conv(len);
                            } else if index.byte() > start {
                                index.byte = start.cast();
                            }
                        };
                        adjust(&mut common.selection.cursor);
                        adjust(&mut common.selection.anchor);
                    } else {
                        log::warn!("buggy IME tried to delete range not at char boundary");
                    }
                }

                if after_bytes > 0 {
                    let start = edit_range.end;
                    let end = start + after_bytes;
                    if self.as_str().is_char_boundary(end) {
                        self.replace_range(start..end, "");
                    } else {
                        log::warn!("buggy IME tried to delete range not at char boundary");
                    }
                }

                if let Some(text) = self.ime_surrounding_text(common) {
                    cx.update_ime_surrounding_text(&common.id, text);
                }

                EventAction::Used
            }
        }
    }
}

impl Common {
    /// Replace a section of text
    ///
    /// Returns the index of the end of the replacement.
    #[inline]
    fn replace_range(
        &mut self,
        parts: &mut impl PartList,
        range: Range<TextIndex>,
        replace_with: &str,
    ) -> TextIndex {
        debug_assert!(range.start <= range.end);
        if !parts.variable_length() {
            let range = subrange_of(&range, 0);
            parts.get_mut(0).replace_range(range.clone(), replace_with);
            return TextIndex::new(0, range.start + replace_with.len());
        }

        let mut p = range.start.part();
        let p_end = range.end.part();
        let mut b_start = range.start.byte();
        let mut last_line_end = 0;
        for line_range in kas::text::LineIterator::new(replace_with) {
            let line = &replace_with[line_range];
            last_line_end = b_start + line.len();
            if p < p_end {
                let part = parts.get_mut(p);
                let b_end = if p + 1 != p_end {
                    part.text.len()
                } else {
                    range.end.byte()
                };
                part.replace_range(b_start..b_end, line);
            } else {
                parts.insert(p, Part {
                    text: Rc::new(line.to_string()),
                    ..Default::default()
                });
            }
            p += 1;
            b_start = 0;
        }

        let p_repl_end = p;

        while p < p_end {
            parts.delete(p);
            p += 1;
        }

        TextIndex::new(p_repl_end, last_line_end)
    }

    /// Fully prepare text for display, ensuring the cursor is within view
    ///
    /// This method performs all required steps of preparation according to the
    /// [`Status`] (which is advanced to [`Status::Ready`]). This method should
    /// be called after changes to the text, alignment or wrap-width.
    #[inline]
    pub fn prepare_and_scroll<H: Highlighter>(
        &mut self,
        parts: &mut impl PartList,
        highlighter: &mut H,
        cx: &mut EventCx,
    ) {
        let mut any_resized = false;

        for part in parts.iter_mut() {
            if !part.is_prepared() {
                part.prepare_runs(self, highlighter);
                any_resized |= part.prepare_wrap(self);
            }
        }

        if any_resized {
            cx.resize();
            self.set_view_offset_from_cursor(parts, cx);
        }
        cx.redraw();
    }

    fn copy_selection_to_string(&self, parts: &impl PartList) -> String {
        let range = self.selection.to_range();
        if range.start.part == range.end.part {
            return parts.get(range.start.part()).as_str()[range.start.byte()..range.end.byte()]
                .to_string();
        }

        let (p_start, p_end) = (range.start.part(), range.end.part());
        let mut c = range.end.byte();
        for p in p_start..p_end {
            c += parts.get(p).as_str().len();
        }
        c -= range.start.byte();

        let mut s = String::with_capacity(c);
        s.push_str(&parts.get(p_start).as_str()[range.start.byte()..]);
        for p in p_start + 1..p_end {
            s.push_str(parts.get(p).as_str());
        }
        if range.end.byte > 0 {
            s.push_str(&parts.get(p_end).as_str()[..range.end.byte()]);
        }

        s
    }

    /// Get the [`TextIndex`] nearest to `coord` within `parts`
    fn text_index_nearest(&self, parts: &impl PartList, coord: Coord) -> TextIndex {
        let mut l_bound = 0;
        let mut u_bound = parts.len();
        let mut p = self.selection.cursor.part();
        let mut best_dist = i32::MAX;
        let mut best_p = p;
        loop {
            let part = parts.get(p);
            debug_assert!(part.is_prepared());
            let (y0, y1) = (part.rect.pos.1, part.rect.pos2().1 - 1);

            let dist = y0.saturating_sub(coord.1).max(coord.1.saturating_sub(y1));
            if dist < best_dist {
                best_dist = dist;
                best_p = p;
            }

            if coord.1 < y0 {
                u_bound = p;
            } else if y1 < coord.1 {
                l_bound = p;
            }
            let q = l_bound + (u_bound - l_bound) / 2;
            if p == q {
                break;
            } else {
                p = q;
            }
        }

        let part = parts.get(best_p);
        let rel_pos = (coord - part.rect().pos).cast();
        let byte = part.display.text_index_nearest(rel_pos);
        TextIndex::new(p, byte)
    }

    /// Get the part used by IME operations
    fn ime_part<'p>(&self, parts: &'p mut impl PartList) -> Option<&'p mut Part> {
        if let Some(part) = self.current.ime_part() {
            Some(parts.get_mut(part))
        } else {
            None
        }
    }

    /// Handle an event
    ///
    /// If [`EventAction::requires_repreparation`] then the caller **must** call
    /// re-prepare the text by calling [`Common::prepare_and_scroll`].
    //
    // TODO(opt): should we use dyn PartList to reduce code size?
    pub fn handle_event(
        &mut self,
        parts: &mut impl PartList,
        cx: &mut EventCx,
        event: Event,
    ) -> EventAction {
        let mut event_action = EventAction::Used;
        let range = match event {
            Event::NavFocus(source) if source == FocusSource::Key => {
                if !self.input_handler.is_selecting() {
                    self.request_key_focus(cx, source);
                }
                return EventAction::Used;
            }
            Event::NavFocus(_) => return EventAction::Used,
            Event::LostNavFocus => return EventAction::Used,
            Event::SelFocus(source) => {
                // NOTE: sel focus implies key focus since we only request
                // the latter. We must set before calling self.set_primary.
                self.has_key_focus = true;
                if source == FocusSource::Pointer {
                    self.set_primary(parts, cx);
                }

                return EventAction::Used;
            }
            Event::KeyFocus => {
                self.has_key_focus = true;
                self.set_view_offset_from_cursor(parts, cx);

                return if self.current.is_none() {
                    let hint = Default::default();
                    let purpose = ImePurpose::Normal;
                    let part = parts.get_mut(self.selection.cursor.part());
                    let surrounding_text = part.ime_surrounding_text(self);
                    cx.replace_ime_focus(self.id.clone(), hint, purpose, surrounding_text);
                    EventAction::FocusGained
                } else {
                    EventAction::Used
                };
            }
            Event::LostKeyFocus => {
                self.has_key_focus = false;
                cx.redraw();
                return if self.current.is_ime_enabled() {
                    EventAction::FocusLost
                } else {
                    EventAction::Used
                };
            }
            Event::LostSelFocus => {
                // NOTE: we can assume that we will receive Ime::Disabled if IME is active
                if !self.selection.is_empty() {
                    self.save_undo_state(parts, None);
                    self.selection.clear_selection();
                }
                self.input_handler.stop_selecting();
                cx.redraw();
                return EventAction::Used;
            }
            Event::Command(cmd, code) => match self.cmd_action(parts, cx, cmd, code) {
                Ok(action) => {
                    if matches!(action, EventAction::Cursor) {
                        self.set_view_offset_from_cursor(parts, cx);
                    }
                    return action;
                }
                Err(NotReady) => return EventAction::Used,
            },
            Event::Key(event, false) if event.state == ElementState::Pressed && !self.read_only => {
                return if let Some(text) = &event.text {
                    let selection = self.selection.to_range();
                    self.save_undo_state(
                        parts,
                        Some(EditOp::KeyInput(
                            selection.start.part(),
                            selection.end.part(),
                        )),
                    );
                    self.cancel_selection_and_ime(parts, cx);

                    let end = self.replace_range(parts, selection.clone(), text);
                    self.selection.set_position(end);
                    self.edit_x_coord = None;

                    EventAction::Edit
                } else {
                    let opt_cmd = cx
                        .config()
                        .shortcuts()
                        .try_match_event(cx.modifiers(), event);
                    if let Some(cmd) = opt_cmd {
                        match self.cmd_action(parts, cx, cmd, Some(event.physical_key)) {
                            Ok(action) => {
                                if matches!(action, EventAction::Cursor) {
                                    self.set_view_offset_from_cursor(parts, cx);
                                }
                                action
                            }
                            Err(NotReady) => EventAction::Used,
                        }
                    } else {
                        EventAction::Unused
                    }
                };
            }
            Event::Ime(ime) => {
                let p = self.selection.cursor.part();
                match self.current {
                    CurrentAction::None if ime == Ime::Enabled => {
                        self.current = CurrentAction::ImeStart(p.cast());
                        parts.get(p).set_ime_cursor_area(self, cx);

                        return if !self.has_key_focus {
                            EventAction::FocusGained
                        } else {
                            EventAction::Used
                        };
                    }
                    CurrentAction::Selection => {
                        cx.cancel_ime_focus(&self.id);
                        return EventAction::Unused;
                    }
                    _ => (),
                }

                if let Some(opt_op) = match ime {
                    Ime::Enabled | Ime::Disabled => None,
                    Ime::Preedit { .. } | Ime::DeleteSurrounding { .. } => Some(None),
                    Ime::Commit { .. } => Some(Some(EditOp::Ime(p))),
                } {
                    self.save_undo_state(parts, opt_op);
                }

                return parts.get_mut(p).handle_ime(self, cx, ime);
            }
            Event::PressStart(press) if press.is_tertiary() => {
                return match press.grab_click(self.id.clone()).complete(cx) {
                    Unused => EventAction::Unused,
                    Used => EventAction::Used,
                };
            }
            Event::PressEnd { press, .. } if press.is_tertiary() => {
                let mut cursor = self.text_index_nearest(parts, press.coord);
                self.cancel_selection_and_ime(parts, cx);
                self.request_key_focus(cx, FocusSource::Pointer);

                if let Some(content) = cx.get_primary() {
                    let p = cursor.part();
                    self.save_undo_state(parts, Some(EditOp::Replace(p, p)));

                    let part = parts.get_mut(p);
                    let range = part.trim_paste(self.wrap, &content);
                    cursor = self.replace_range(parts, cursor..cursor, &content[range.clone()]);
                    event_action = EventAction::Edit;
                }
                cursor.into()
            }
            event => match self.input_handler.handle(cx, self.id.clone(), event) {
                TextInputAction::Used => return EventAction::Used,
                TextInputAction::Unused => return EventAction::Unused,
                TextInputAction::PressStart {
                    coord,
                    clear,
                    repeats,
                } => {
                    if let Some(part) = self.ime_part(parts) {
                        part.clear_ime(self);
                        cx.cancel_ime_focus(&self.id);
                    }
                    self.request_key_focus(cx, FocusSource::Pointer);
                    self.save_undo_state(parts, Some(EditOp::Cursor));
                    self.current = CurrentAction::Selection;

                    let mut cursor = self.text_index_nearest(parts, coord);
                    let mut anchor = if clear { cursor } else { self.selection.anchor };

                    if repeats > 1 {
                        if anchor.part == cursor.part {
                            let part = parts.get(cursor.part());
                            let r = TextInput::expand_range(
                                part.text.as_str(),
                                CursorRange::from(anchor.byte()..cursor.byte()),
                                (repeats >= 3)
                                    .then_some(&|index| part.display.find_line(index).map(|r| r.1)),
                            );
                            anchor.byte = r.anchor.cast();
                            cursor.byte = r.cursor.cast();
                        } else {
                            // TODO: anchor and cursor use different parts; expand separately then recombine
                        }
                    }
                    CursorRange::from(anchor..cursor)
                }
                TextInputAction::PressMove { coord, repeats } => {
                    if self.current != CurrentAction::Selection {
                        return EventAction::Used;
                    }

                    let mut anchor = self.selection.anchor;
                    let mut cursor = self.selection.cursor;
                    let index = self.text_index_nearest(parts, coord);
                    if index.part == anchor.part && index.part == cursor.part {
                        let part = parts.get(index.part());
                        let r = TextInput::adjust_range(
                            part.text.as_str(),
                            CursorRange::from(anchor.byte()..cursor.byte()),
                            index.byte(),
                            repeats,
                            Some(&|index| part.display.find_line(index).map(|r| r.1)),
                        );
                        anchor.byte = r.anchor.cast();
                        cursor.byte = r.cursor.cast();
                    } else {
                        // TODO
                        cursor = index;
                    }
                    CursorRange::from(anchor..cursor)
                }
                TextInputAction::PressEnd { coord } => {
                    if let Some(part) = self.ime_part(parts) {
                        part.clear_ime(self);
                        cx.cancel_ime_focus(&self.id);
                    }
                    self.save_undo_state(parts, Some(EditOp::Cursor));
                    if self.current == CurrentAction::Selection {
                        self.set_primary(parts, cx);
                    } else {
                        let index = self.text_index_nearest(parts, coord);
                        self.selection.cursor = index;
                        self.selection.clear_selection();
                    }
                    self.current = CurrentAction::None;

                    self.request_key_focus(cx, FocusSource::Pointer);
                    return EventAction::Used;
                }
            },
        };

        if range != self.selection {
            self.selection = range;
            self.set_view_offset_from_cursor(parts, cx);
            self.edit_x_coord = None;
            cx.redraw();
        }
        event_action
    }

    /// Cancel on-going selection and IME actions
    ///
    /// This should be called if e.g. key-input interrupts the current
    /// action.
    fn cancel_selection_and_ime(&mut self, parts: &mut impl PartList, cx: &mut EventState) {
        if self.current == CurrentAction::Selection {
            self.input_handler.stop_selecting();
            self.current = CurrentAction::None;
        } else if let Some(part) = self.ime_part(parts) {
            part.clear_ime(self);
            cx.cancel_ime_focus(&self.id);
        }
    }

    /// Call before an edit to (potentially) commit current state based on last_edit
    ///
    /// Call with [`None`] to force commit of any uncommitted changes.
    fn save_undo_state(&mut self, parts: &mut impl PartList, edit: Option<EditOp>) {
        if let Some(op) = edit
            && op.try_merge(&mut self.last_edit)
        {
            return;
        }

        self.last_edit = edit;
        let (part, texts) = match edit {
            None | Some(EditOp::Initial) | Some(EditOp::Cursor) => (0, vec![]),
            Some(EditOp::Ime(part)) => (part, vec![Rc::clone(&parts.get(part).text)]),
            Some(EditOp::KeyInput(start, last))
            | Some(EditOp::KeyDelete(start, last))
            | Some(EditOp::Replace(start, last)) => {
                let texts = (start..last + 1)
                    .map(|part| Rc::clone(&parts.get(part).text))
                    .collect();
                (start, texts)
            }
        };
        self.undo_stack
            .try_push((part, parts.len(), texts, self.selection));
    }

    /// Request key focus, if we don't have it or IME
    fn request_key_focus(&self, cx: &mut EventCx, source: FocusSource) {
        if !self.has_key_focus && !self.current.is_ime_enabled() {
            cx.request_key_focus(self.id.clone(), source);
        }
    }
    /// Drive action of a [`Command`]
    fn cmd_action(
        &mut self,
        parts: &mut impl PartList,
        cx: &mut EventCx,
        mut cmd: Command,
        code: Option<PhysicalKey>,
    ) -> Result<EventAction, NotReady> {
        let editable = !self.read_only;
        let mut shift = cx.modifiers().shift_key();
        let mut buf = [0u8; 4];
        let cursor = self.selection.cursor;
        let c_p = cursor.part();
        let cursor = cursor.byte();
        let c_part = parts.get(c_p);
        debug_assert!(c_part.is_prepared());
        let c_part_len = c_part.as_str().len();
        let multi_line = self.wrap;
        let selection = self.selection.to_range();
        let have_sel = selection.end > selection.start;
        let string;

        if c_part.text_is_rtl() {
            match cmd {
                Command::Left => cmd = Command::Right,
                Command::Right => cmd = Command::Left,
                Command::WordLeft => cmd = Command::WordRight,
                Command::WordRight => cmd = Command::WordLeft,
                _ => (),
            };
        }

        enum Action<'a> {
            Deselect,
            Activate,
            // bool in Insert, Delete indices "is key input" (i.e. undo op is mergeable)
            Insert(&'a str, bool),
            Delete(Range<TextIndex>, bool),
            Move(TextIndex, Option<f32>),
            UndoRedo(bool),
        }

        let action = match cmd {
            Command::Escape | Command::Deselect if !selection.is_empty() => Action::Deselect,
            Command::Activate => Action::Activate,
            Command::Enter if shift || !multi_line => Action::Activate,
            Command::Enter if editable && multi_line => {
                Action::Insert('\n'.encode_utf8(&mut buf), true)
            }
            // NOTE: we might choose to optionally handle Tab in the future,
            // but without some workaround it prevents keyboard navigation.
            // Command::Tab => Action::Insert('\t'.encode_utf8(&mut buf), true),
            Command::Left | Command::Home if !shift && have_sel => {
                Action::Move(selection.start, None)
            }
            Command::Left => {
                let text;
                let mut p = c_p;
                let mut cursor = cursor;
                if cursor > 0 {
                    text = c_part.as_str();
                } else if p > 0 {
                    p -= 1;
                    text = parts.get(p).as_str();
                    cursor = text.len();
                } else {
                    return Ok(EventAction::Used);
                };

                let byte = GraphemeCursor::new(cursor, text.len(), true)
                    .prev_boundary(text, 0)
                    .unwrap()
                    .unwrap_or(0);
                Action::Move(TextIndex::new(p, byte), None)
            }
            Command::Right | Command::End if !shift && have_sel => {
                Action::Move(selection.end, None)
            }
            Command::Right => {
                if cursor < c_part_len {
                    let byte = GraphemeCursor::new(cursor, c_part_len, true)
                        .next_boundary(c_part.as_str(), 0)
                        .unwrap()
                        .unwrap_or(c_part_len);
                    Action::Move(TextIndex::new(c_p, byte), None)
                } else {
                    let p = c_p + 1;
                    if p < parts.len() {
                        Action::Move(TextIndex::new(p, 0), None)
                    } else {
                        return Ok(EventAction::Used);
                    }
                }
            }
            Command::WordLeft if cursor > 0 => {
                let mut iter = c_part.as_str()[0..cursor].split_word_bound_indices();
                let mut byte = iter.next_back().map(|(index, _)| index).unwrap_or(0);
                while c_part.as_str()[byte..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
                {
                    if let Some((index, _)) = iter.next_back() {
                        byte = index;
                    } else {
                        break;
                    }
                }
                // TODO: prev
                Action::Move(TextIndex::new(c_p, byte), None)
            }
            Command::WordRight if cursor < c_part_len => {
                let mut iter = c_part.as_str()[cursor..].split_word_bound_indices().skip(1);
                let mut byte = iter
                    .next()
                    .map(|(index, _)| cursor + index)
                    .unwrap_or(c_part_len);
                while c_part.as_str()[byte..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
                {
                    if let Some((index, _)) = iter.next() {
                        byte = cursor + index;
                    } else {
                        break;
                    }
                }
                // TODO: next
                Action::Move(TextIndex::new(c_p, byte), None)
            }
            // Avoid use of unused navigation keys (e.g. by ScrollComponent):
            Command::WordLeft | Command::WordRight => {
                return Ok(EventAction::Used);
            }
            Command::Up | Command::Down if multi_line => {
                let x = match self.edit_x_coord {
                    Some(x) => x,
                    None => c_part
                        .display
                        .text_glyph_pos(cursor)
                        .next_back()
                        .map(|r| r.pos.0)
                        .unwrap_or(0.0),
                };
                let mut line = c_part.display.find_line(cursor).map(|r| r.0).unwrap_or(0);
                // We can tolerate invalid line numbers here!
                line = match cmd {
                    Command::Up => line.wrapping_sub(1),
                    Command::Down => line.wrapping_add(1),
                    _ => unreachable!(),
                };
                const HALF: usize = usize::MAX / 2;
                let nearest_end = match line {
                    0..=HALF => c_part_len,
                    _ => 0,
                };
                // TODO: prev/next
                c_part
                    .display
                    .line_index_nearest(line, x)
                    .map(|index| Action::Move(TextIndex::new(c_p, index), Some(x)))
                    .unwrap_or(Action::Move(TextIndex::new(c_p, nearest_end), None))
            }
            Command::Home if cursor > 0 => {
                // TODO: we don't need to use find_line if each part represents a line
                let index = c_part
                    .display
                    .find_line(cursor)
                    .map(|r| r.1.start)
                    .unwrap_or(0);
                Action::Move(TextIndex::new(c_p, index), None)
            }
            Command::End if cursor < c_part_len => {
                let index = c_part
                    .display
                    .find_line(cursor)
                    .map(|r| r.1.end)
                    .unwrap_or(c_part_len);
                Action::Move(TextIndex::new(c_p, index), None)
            }
            Command::DocHome if c_p > 0 || cursor > 0 => Action::Move(TextIndex::new(0, 0), None),
            Command::DocEnd if c_p + 1 < parts.len() || cursor < c_part_len => {
                let p = parts.len() - 1;
                let len = parts.get(p).as_str().len();
                Action::Move(TextIndex::new(p, len), None)
            }
            // Avoid use of unused navigation keys (e.g. by ScrollComponent):
            Command::Home | Command::End | Command::DocHome | Command::DocEnd => {
                return Ok(EventAction::Used);
            }
            Command::PageUp | Command::PageDown if multi_line => {
                let mut v = c_part
                    .display
                    .text_glyph_pos(cursor)
                    .next_back()
                    .map(|r| r.pos.into())
                    .unwrap_or(Vec2::ZERO);
                if let Some(x) = self.edit_x_coord {
                    v.0 = x;
                }
                // TODO: page height should be an input?
                let mut line_height = self.dpem;
                if let Some(line) = c_part.display.lines().next() {
                    line_height = line.bottom() - line.top();
                }
                let mut h_dist = line_height * 10.0;
                if cmd == Command::PageUp {
                    h_dist *= -1.0;
                }
                v.1 += h_dist;
                let pos = c_part.rect.pos + Offset::conv_nearest(v);
                let index = self.text_index_nearest(parts, pos).byte();
                Action::Move(TextIndex::new(c_p, index), Some(v.0))
            }
            Command::Delete | Command::DelBack if editable && have_sel => {
                Action::Delete(selection.clone(), true)
            }
            Command::Delete if editable => {
                if let Some(action) = GraphemeCursor::new(cursor, c_part_len, true)
                    .next_boundary(c_part.as_str(), 0)
                    .unwrap()
                    .map(|next| {
                        Action::Delete(self.selection.cursor..TextIndex::new(c_p, next), true)
                    })
                {
                    action
                } else {
                    return Ok(EventAction::Used);
                }
            }
            Command::DelBack if editable => {
                if let Some(action) = GraphemeCursor::new(cursor, c_part_len, true)
                    .prev_boundary(c_part.as_str(), 0)
                    .unwrap()
                    .map(|prev| {
                        Action::Delete(TextIndex::new(c_p, prev)..self.selection.cursor, true)
                    })
                {
                    action
                } else {
                    return Ok(EventAction::Used);
                }
            }
            Command::DelWord if editable => {
                let next = c_part.as_str()[cursor..]
                    .split_word_bound_indices()
                    .nth(1)
                    .map(|(index, _)| cursor + index)
                    .unwrap_or(c_part_len);
                Action::Delete(self.selection.cursor..TextIndex::new(c_p, next), true)
            }
            Command::DelWordBack if editable => {
                let prev = c_part.as_str()[0..cursor]
                    .split_word_bound_indices()
                    .next_back()
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                Action::Delete(TextIndex::new(c_p, prev)..self.selection.cursor, true)
            }
            Command::SelectAll => {
                self.selection.anchor = TextIndex::new(0, 0);
                shift = true; // hack
                let p = parts.len() - 1;
                let len = parts.get(p).as_str().len();
                Action::Move(TextIndex::new(p, len), None)
            }
            Command::Cut | Command::Copy if have_sel => {
                let text = self.copy_selection_to_string(parts);
                cx.set_clipboard(text);
                if cmd == Command::Cut && editable {
                    Action::Delete(selection.clone(), false)
                } else {
                    return Ok(EventAction::Used);
                }
            }
            Command::Paste if editable => {
                if let Some(content) = cx.get_clipboard() {
                    let range = c_part.trim_paste(self.wrap, &content);
                    string = content;
                    Action::Insert(&string[range], false)
                } else {
                    return Ok(EventAction::Used);
                }
            }
            Command::Undo | Command::Redo if editable => Action::UndoRedo(cmd == Command::Redo),
            _ => return Ok(EventAction::Unused),
        };

        // We can receive some commands without key focus as a result of
        // selection focus. Request focus on edit actions (like Command::Cut).
        if !matches!(action, Action::Deselect) {
            self.request_key_focus(cx, FocusSource::Synthetic);
        }
        self.cancel_selection_and_ime(parts, cx);

        let edit_op = match action {
            Action::Deselect | Action::Move(_, _) => Some(EditOp::Cursor),
            Action::Activate | Action::UndoRedo(_) => None,
            Action::Insert(_, false) => Some(EditOp::Replace(
                selection.start.part(),
                selection.end.part(),
            )),
            Action::Insert(_, true) => Some(EditOp::KeyInput(
                selection.start.part(),
                selection.end.part(),
            )),
            Action::Delete(ref range, false) => {
                Some(EditOp::Replace(range.start.part(), range.end.part()))
            }
            Action::Delete(ref range, true) => {
                Some(EditOp::KeyDelete(range.start.part(), range.end.part()))
            }
        };
        self.save_undo_state(parts, edit_op);

        let action = match action {
            Action::Deselect => {
                self.selection.clear_selection();
                cx.redraw();
                EventAction::Cursor
            }
            Action::Activate => EventAction::Activate(code),
            Action::Insert(s, _) => {
                let mut index = self.selection.cursor;
                let range = if have_sel { selection.clone() } else { index..index };
                index = self.replace_range(parts, range, s);
                self.selection.set_position(index);
                self.edit_x_coord = None;
                EventAction::Edit
            }
            Action::Delete(sel, _) => {
                self.replace_range(parts, sel.clone(), "");
                self.selection.set_position(sel.start);
                self.edit_x_coord = None;
                EventAction::Edit
            }
            Action::Move(index, x_coord) => {
                self.selection.cursor = index;
                if !shift {
                    self.selection.clear_selection();
                } else {
                    self.set_primary(parts, cx);
                }
                self.edit_x_coord = x_coord;
                cx.redraw();
                EventAction::Cursor
            }
            Action::UndoRedo(redo) => {
                if let Some((p, old_num_parts, texts, cursor)) = self.undo_stack.undo_or_redo(redo)
                {
                    let mut p = *p;
                    let mut n = 0;
                    if parts.len() < *old_num_parts {
                        n = old_num_parts - parts.len();
                        for text in &texts[..n] {
                            let part = Part {
                                text: Rc::clone(text),
                                ..Default::default()
                            };
                            parts.insert(p, part);
                            p += 1;
                        }
                    } else if parts.len() > *old_num_parts {
                        let mut n_delete = parts.len() - old_num_parts;
                        while n_delete > 0 {
                            parts.delete(p);
                            n_delete -= 1;
                        }
                    }

                    for text in &texts[n..] {
                        let part = parts.get_mut(p);
                        if !Rc::ptr_eq(&part.text, text) {
                            part.text = Rc::clone(text);
                            part.status = Status::New;
                        }
                        p += 1;
                    }

                    self.edit_x_coord = None;
                    self.selection = *cursor;
                    EventAction::Edit
                } else {
                    EventAction::Used
                }
            }
        };

        Ok(action)
    }

    /// Set primary clipboard (mouse buffer) contents from selection
    fn set_primary(&self, parts: &impl PartList, cx: &mut EventCx) {
        if self.has_key_focus && !self.selection.is_empty() && cx.has_primary() {
            cx.set_primary(self.copy_selection_to_string(parts));
        }
    }

    /// Update view_offset after the cursor index changes
    ///
    /// It is assumed that the text has not changed.
    ///
    /// A redraw is assumed since the cursor moved.
    fn set_view_offset_from_cursor(&self, parts: &impl PartList, cx: &mut EventCx) {
        let cursor = self.selection.cursor;
        let part = parts.get(cursor.part());
        if part.is_prepared()
            && let Some(marker) = part.display.text_glyph_pos(cursor.byte()).next_back()
        {
            let y0 = (marker.pos.1 - marker.ascent).cast_floor();
            let pos = part.rect.pos + Offset(marker.pos.0.cast_nearest(), y0);
            let size = Size(0, i32::conv_ceil(marker.pos.1 - marker.descent) - y0);
            cx.set_scroll(Scroll::Rect(Rect { pos, size }));
        }
    }
}

/// Text editor interface
impl Editor {
    /// Get a reference to the widget's identifier
    #[inline]
    pub fn id_ref(&self) -> &Id {
        &self.common.id
    }

    /// Get the widget's identifier
    #[inline]
    pub fn id(&self) -> Id {
        self.id_ref().clone()
    }

    /// Get text contents
    #[inline]
    pub fn as_str(&self) -> &str {
        self.part.text.as_str()
    }

    /// Get the text contents as a `String`
    #[inline]
    pub fn clone_string(&self) -> String {
        self.as_str().to_string()
    }

    /// Get the (horizontal) text direction
    ///
    /// This returns `true` if the text is inferred to have right-to-left;
    /// in other cases (including when the text is empty) it returns `false`.
    #[inline]
    pub fn text_is_rtl(&self) -> bool {
        self.part.text_is_rtl()
    }

    /// Clear text contents and undo history
    #[inline]
    pub fn clear(&mut self, cx: &mut EventState) {
        self.common.last_edit = Some(EditOp::Initial);
        self.common.undo_stack.clear();
        self.common.cancel_selection_and_ime(&mut self.part, cx);

        Rc::make_mut(&mut self.part.text).clear();
        self.part.require_reprepare();

        self.common.selection.set_max_len(TextIndex::new(0, 0));
        self.common.edit_x_coord = None;
        self.error_state = None;
    }

    /// Set text contents from a `str`
    ///
    /// Returns `true` if the text may have changed.
    #[inline]
    pub fn set_str(&mut self, cx: &mut EventState, text: &str) -> bool {
        if self.as_str() != text {
            self.set_string(cx, text.to_string());
            true
        } else {
            false
        }
    }

    /// Set text contents from a `String`
    ///
    /// This method does not call action handlers on the guard.
    pub fn set_string(&mut self, cx: &mut EventState, text: String) {
        if self.as_str() == text {
            return; // no change
        }

        self.common
            .save_undo_state(&mut self.part, Some(EditOp::Replace(0, 0)));

        self.common.cancel_selection_and_ime(&mut self.part, cx);

        self.part.text = Rc::new(text);
        self.part.require_reprepare();

        let len = TextIndex::new(0, self.as_str().len());
        self.common.selection.set_max_len(len);
        self.common.edit_x_coord = None;
        self.error_state = None;
    }

    /// Replace selected text
    ///
    /// This method does not call action handlers on the guard.
    #[inline]
    pub fn replace_selected_text(&mut self, cx: &mut EventState, text: &str) {
        self.common
            .save_undo_state(&mut self.part, Some(EditOp::Replace(0, 0)));

        self.common.cancel_selection_and_ime(&mut self.part, cx);

        let selection = self.common.selection.to_range();
        let start = selection.start.byte();
        let end = selection.end.byte();
        self.part.replace_range(start..end, text);
        let index = TextIndex::new(0, start + text.len());
        self.common.selection.set_position(index);
        self.error_state = None;
    }

    /// Access the cursor index / selection range
    #[inline]
    pub fn cursor_range(&self) -> CursorRange<usize> {
        CursorRange {
            anchor: self.common.selection.anchor.byte(),
            cursor: self.common.selection.cursor.byte(),
        }
    }

    /// Set the cursor index / range
    ///
    /// This does not interact with undo history or call action handlers on the
    /// guard.
    #[inline]
    pub fn set_cursor_range(&mut self, range: CursorRange<usize>) {
        self.common.edit_x_coord = None;
        self.common.selection = CursorRange {
            anchor: TextIndex::new(0, range.anchor),
            cursor: TextIndex::new(0, range.cursor),
        };
    }

    /// Get whether this text-edit widget is read-only
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.common.read_only
    }

    /// Set whether this text-edit widget is editable
    #[inline]
    pub fn set_read_only(&mut self, read_only: bool) {
        self.common.read_only = read_only;
    }

    /// True if the editor uses multi-line mode
    #[inline]
    pub fn multi_line(&self) -> bool {
        self.common.wrap
    }

    /// Get whether the widget has input focus
    ///
    /// This is true when the widget is has keyboard or IME focus.
    #[inline]
    pub fn has_input_focus(&self) -> bool {
        self.common.has_key_focus || self.common.current.is_ime_enabled()
    }

    /// Get whether the input state is erroneous
    #[inline]
    pub fn has_error(&self) -> bool {
        self.error_state.is_some()
    }

    /// Get the error message, if any
    #[inline]
    pub fn error_message(&self) -> Option<&str> {
        self.error_state.as_ref().and_then(|state| state.as_deref())
    }

    /// Mark the input as erroneous with an optional message
    ///
    /// This state should be set from [`EditGuard::edit`] when appropriate. The
    /// state is cleared immediately before calling [`EditGuard::edit`] and also
    /// in case a text is directly assigned (e.g. using [`Self::set_string`]).
    ///
    /// When set, the input field's background is drawn red. If a message is
    /// supplied, then a tooltip will be available on mouse-hover.
    pub fn set_error(&mut self, cx: &mut EventState, message: Option<Cow<'static, str>>) {
        self.error_state = Some(message);
        cx.redraw(self.id_ref());
    }
}
