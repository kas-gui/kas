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
use kas::text::{CursorRange, Direction, NotReady, SelectionHelper, Status, TextDisplay, format};
use kas::theme::{Background, DrawCx, SizeCx, TextClass};
use kas::util::UndoStack;
use kas::{Layout, autoimpl};
use std::borrow::Cow;
use std::num::NonZeroUsize;
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

/// Editor state common to all parts
#[derive(Debug, Default)]
pub struct Common<H: Highlighter> {
    colors: SchemeColors,
    highlighter: H,
}

impl<H: Highlighter> Common<H> {
    /// Replace the highlighter
    #[inline]
    pub fn with_highlighter<H2: Highlighter>(self, highlighter: H2) -> Common<H2> {
        Common {
            colors: SchemeColors::default(),
            highlighter,
        }
    }

    /// Set a new highlighter of the same type
    ///
    /// Also call [`Part::require_reprepare`]()
    /// on each part to ensure the highlighting is updated.
    pub fn set_highlighter(&mut self, highlighter: H) {
        self.highlighter = highlighter;
    }

    /// Configure `Common` data
    #[inline]
    #[must_use]
    pub fn configure(&mut self, cx: &mut ConfigCx) -> Option<ActionResetStatus> {
        if self.highlighter.configure(cx) {
            self.colors = self.highlighter.scheme_colors();
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
    // TODO(opt): id is duplicated here since macros don't let us put the core here
    id: Id,
    font: FontSelector,
    dpem: f32,
    direction: Direction,
    wrap: bool,
    read_only: bool,
    rect: Rect,
    status: Status,
    display: TextDisplay,
    highlight: highlight::Cache,
    text: String,
    selection: SelectionHelper,
    edit_x_coord: Option<f32>,
    last_edit: Option<EditOp>,
    undo_stack: UndoStack<(String, CursorRange)>,
    has_key_focus: bool,
    current: CurrentAction,
    input_handler: TextInput,
}

/// Inner editor interface
///
/// This type provides an API usable by [`EditGuard`] and (read-only) via
/// [`Deref`](std::ops::Deref) from [`EditBoxCore`] and [`EditBox`].
#[autoimpl(Debug)]
pub struct Editor {
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
pub struct Component<H: Highlighter>(pub Editor, pub Common<H>);

impl<H: Highlighter> Layout for Component<H> {
    #[inline]
    fn rect(&self) -> Rect {
        self.0.part.rect
    }

    #[inline]
    fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
        self.0.part.size_rules(cx, axis)
    }

    #[inline]
    fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, _: AlignHints) {
        self.0.part.set_rect(cx, rect);
    }

    #[inline]
    fn draw(&self, draw: DrawCx) {
        self.0
            .part
            .draw_with_offset(draw, &self.1.colors, self.rect(), Offset::ZERO);
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
            part: Part::new(wrap),
            error_state: None,
        };
        Component(editor, Common::default())
    }

    /// Set whether long lines are automatically wrapped
    #[inline]
    pub fn set_wrap(&mut self, wrap: bool) {
        self.0.part.wrap = wrap;
        self.0.part.status = Status::New;
    }

    /// Set the base text direction
    #[inline]
    pub fn set_direction(&mut self, direction: Direction) {
        self.0.part.set_direction(direction);
    }

    /// Replace the highlighter
    #[inline]
    pub fn with_highlighter<H2: Highlighter>(self, highlighter: H2) -> Component<H2> {
        let common = Common {
            colors: self.1.colors,
            highlighter,
        };
        Component(self.0, common)
    }

    /// Set a new highlighter of the same type
    pub fn set_highlighter(&mut self, highlighter: H) {
        self.1.highlighter = highlighter;
        self.0.part.require_reprepare();
    }

    /// Get the background color
    ///
    /// Uses the UI theme's error color if applicable.
    pub fn background_color(&self) -> Background {
        if self.0.error_state.is_some() {
            Background::Error
        } else {
            self.1.background_color()
        }
    }

    /// Set the initial text (inline)
    ///
    /// This method should only be used on a new `Component`.
    #[inline]
    #[must_use]
    pub fn with_text(mut self, text: impl ToString) -> Self {
        self.0.part = self.0.part.with_text(text);
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
        if let Some(ActionResetStatus) = self.1.configure(cx) {
            self.0.part.require_reprepare();
        }
        self.0.part.configure(&mut self.1, cx, id);
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

        self.0.part.prepare_runs(&mut self.1);
        self.0.part.prepare_wrap();
    }

    /// Fully prepare text for display, ensuring the cursor is within view
    ///
    /// This method performs all required steps of preparation according to the
    /// [`Status`] (which is advanced to [`Status::Ready`]). This method should
    /// be called after changes to the text, alignment or wrap-width.
    #[inline]
    pub fn prepare_and_scroll(&mut self, cx: &mut EventCx) {
        self.0.part.prepare_and_scroll(&mut self.1, cx);
    }

    /// Measure required vertical height, wrapping as configured
    ///
    /// Stops after `max_lines`, if provided.
    ///
    /// May partially prepare the text for display, but does not otherwise
    /// modify `self`.
    #[inline]
    pub fn measure_height(&mut self, wrap_width: f32, max_lines: Option<NonZeroUsize>) -> f32 {
        self.0.part.prepare_runs(&mut self.1);
        self.0.part.display.measure_height(wrap_width, max_lines)
    }

    /// Implementation of [`Viewport::draw_with_offset`]
    #[inline]
    pub fn draw_with_offset(&self, draw: DrawCx, rect: Rect, offset: Offset) {
        self.0
            .part
            .draw_with_offset(draw, &self.1.colors, rect, offset);
    }

    /// Handle an event
    #[inline]
    pub fn handle_event(&mut self, cx: &mut EventCx, event: Event) -> EventAction {
        let action = self.0.part.handle_event(cx, event);
        if action.requires_repreparation() {
            self.0.part.prepare_and_scroll(&mut self.1, cx);
        }
        action
    }

    /// Clear the error state
    #[inline]
    pub fn clear_error(&mut self) {
        self.0.error_state = None;
    }
}

impl Part {
    /// Construct a new instance
    #[inline]
    pub fn new(wrap: bool) -> Self {
        Part {
            id: Id::default(),
            font: FontSelector::default(),
            dpem: 16.0,
            direction: Direction::Auto,
            wrap,
            read_only: false,
            rect: Rect::ZERO,
            status: Status::New,
            display: TextDisplay::default(),
            highlight: Default::default(),
            text: Default::default(),
            selection: Default::default(),
            edit_x_coord: None,
            last_edit: Some(EditOp::Initial),
            undo_stack: UndoStack::new(),
            has_key_focus: false,
            current: CurrentAction::None,
            input_handler: Default::default(),
        }
    }

    /// Set the base text direction
    #[inline]
    pub fn set_direction(&mut self, direction: Direction) {
        self.direction = direction;
        self.status = Status::New;
    }

    /// Set the initial text (inline)
    ///
    /// This method should only be used on a new `Part`.
    #[inline]
    #[must_use]
    pub fn with_text(mut self, text: impl ToString) -> Self {
        debug_assert!(self.current == CurrentAction::None && !self.input_handler.is_selecting());
        let text = text.to_string();
        let len = text.len();
        self.text = text;
        self.selection.set_cursor(len);
        self
    }

    /// Get text contents
    #[inline]
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    /// Get the base directionality of the text
    ///
    /// [`Self::configure`] should be called before this method.
    #[inline]
    pub fn text_is_rtl(&self) -> bool {
        debug_assert!(self.status >= Status::ResizeLevelRuns);
        self.display.text_is_rtl()
    }

    /// Access the cursor index / selection range
    #[inline]
    pub fn cursor_range(&self) -> CursorRange {
        *self.selection
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

    /// Configure component
    ///
    /// [`Common::configure`] must be called before this method.
    pub fn configure<H: Highlighter>(&mut self, common: &mut Common<H>, cx: &mut ConfigCx, id: Id) {
        self.id = id;
        let cx = cx.size_cx();
        let font = cx.font(TextClass::Editor);
        let dpem = cx.dpem(TextClass::Editor);
        if font != self.font {
            self.font = font;
            self.dpem = dpem;
            self.status = Status::New;
        } else if dpem != self.dpem {
            self.dpem = dpem;
            self.status = self.status.min(Status::ResizeLevelRuns);
        }
        self.prepare_runs(common);
    }

    /// Perform run-breaking and shaping
    ///
    /// This represents a high-level step of preparation required before
    /// displaying text. After the `Part` is [configured](Self::configure), this
    /// method should be called before any sizing operations. This will advance
    /// the [`Status`] to [`Status::LevelRuns`].
    /// This method must be called again after any edits to the `Part`'s text.
    #[inline]
    pub fn prepare_runs<H: Highlighter>(&mut self, common: &mut Common<H>) {
        fn inner<H: Highlighter>(part: &mut Part, common: &mut Common<H>) {
            part.highlight
                .highlight(&part.text, &mut common.highlighter);

            let text = part.text.as_str();
            let font_tokens = part.highlight.font_tokens(part.dpem, part.font);
            match part.status {
                Status::New => part
                    .display
                    .prepare_runs(text, part.direction, font_tokens)
                    .expect("no suitable font found"),
                Status::ResizeLevelRuns => part.display.resize_runs(text, font_tokens),
                _ => return,
            }

            part.status = Status::LevelRuns;
        }

        if self.status < Status::LevelRuns {
            inner(self, common);
        }
    }

    /// Get the assigned [`Rect`]
    #[inline]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Solve size rules
    pub fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
        let rules = if axis.is_horizontal() {
            let mut bound = 0i32;
            if self.wrap {
                let (min, ideal) = cx.wrapped_line_len(TextClass::Editor, self.dpem);
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
            let wrap_width = self
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
    pub fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect) {
        if rect.size.0 != self.rect.size.0 {
            self.status = self.status.min(Status::LevelRuns);
        }
        self.rect = rect;

        self.prepare_wrap();
        if self.current.is_ime_enabled() {
            self.set_ime_cursor_area(cx);
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
    fn prepare_wrap(&mut self) -> bool {
        if self.status < Status::LevelRuns || self.rect.size.0 == 0 {
            return false;
        };

        let bb = self.display.bounding_box();

        if self.status == Status::LevelRuns {
            let align_width = self.rect.size.0.cast();
            let wrap_width = if !self.wrap { f32::INFINITY } else { align_width };
            self.display
                .prepare_lines(wrap_width, align_width, Align::Default);
            self.display.ensure_non_negative_alignment();
        }

        self.status = Status::Ready;
        bb != self.display.bounding_box()
    }

    /// Fully prepare text for display, ensuring the cursor is within view
    ///
    /// This method performs all required steps of preparation according to the
    /// [`Status`] (which is advanced to [`Status::Ready`]). This method should
    /// be called after changes to the text, alignment or wrap-width.
    #[inline]
    pub fn prepare_and_scroll<H: Highlighter>(&mut self, common: &mut Common<H>, cx: &mut EventCx) {
        if self.is_prepared() {
            return;
        }

        self.prepare_runs(common);
        if self.prepare_wrap() {
            cx.resize();
            self.set_view_offset_from_cursor(cx);
        }
        cx.redraw();
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
    pub fn draw_with_offset(
        &self,
        mut draw: DrawCx,
        colors: &SchemeColors,
        rect: Rect,
        offset: Offset,
    ) {
        if !self.is_prepared() {
            return;
        }

        let pos = self.rect.pos - offset;
        let range: Range<u32> = self.selection.range().cast();

        let color_tokens = self.highlight.color_tokens();
        let default_colors = format::Colors {
            foreground: colors.foreground,
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
            buf[1].1.foreground = colors.selection_foreground;
            buf[1].1.background = Some(colors.selection_background);
            buf[2].0 = range.end;
            let r0 = if range.start > 0 { 0 } else { 1 };
            &buf[r0..]
        } else {
            let set_selection_colors = |c: &mut format::Colors| {
                if c.foreground == colors.foreground {
                    c.foreground = colors.selection_foreground;
                }
                c.background = Some(colors.selection_background);
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

        if let CurrentAction::ImePreedit { edit_range } = self.current.clone() {
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

        if !self.read_only && draw.ev_state().has_input_focus(&self.id) == Some(true) {
            draw.text_cursor(
                pos,
                rect,
                &self.display,
                self.selection.edit_index(),
                Some(colors.cursor),
            );
        }
    }

    /// Handle an event
    ///
    /// If [`EventAction::requires_repreparation`] then the caller **must** call
    /// re-prepare the text by calling [`Self::prepare_and_scroll`].
    #[inline]
    pub fn handle_event(&mut self, cx: &mut EventCx, event: Event) -> EventAction {
        if !self.is_prepared() {
            debug_assert!(false);
            return EventAction::Unused;
        }

        match event {
            Event::NavFocus(source) if source == FocusSource::Key => {
                if !self.input_handler.is_selecting() {
                    self.request_key_focus(cx, source);
                }
                EventAction::Used
            }
            Event::NavFocus(_) => EventAction::Used,
            Event::LostNavFocus => EventAction::Used,
            Event::SelFocus(source) => {
                // NOTE: sel focus implies key focus since we only request
                // the latter. We must set before calling self.set_primary.
                self.has_key_focus = true;
                if source == FocusSource::Pointer {
                    self.set_primary(cx);
                }

                EventAction::Used
            }
            Event::KeyFocus => {
                self.has_key_focus = true;
                self.set_view_offset_from_cursor(cx);

                if self.current.is_none() {
                    let hint = Default::default();
                    let purpose = ImePurpose::Normal;
                    let surrounding_text = self.ime_surrounding_text();
                    cx.replace_ime_focus(self.id.clone(), hint, purpose, surrounding_text);
                    EventAction::FocusGained
                } else {
                    EventAction::Used
                }
            }
            Event::LostKeyFocus => {
                self.has_key_focus = false;
                cx.redraw();
                if !self.current.is_ime_enabled() {
                    EventAction::FocusLost
                } else {
                    EventAction::Used
                }
            }
            Event::LostSelFocus => {
                // NOTE: we can assume that we will receive Ime::Disabled if IME is active
                if !self.selection.is_empty() {
                    self.save_undo_state(None);
                    self.selection.set_empty();
                }
                self.input_handler.stop_selecting();
                cx.redraw();
                EventAction::Used
            }
            Event::Command(cmd, code) => match self.cmd_action(cx, cmd, code) {
                Ok(action) => {
                    if matches!(action, EventAction::Cursor) {
                        self.set_view_offset_from_cursor(cx);
                    }
                    action
                }
                Err(NotReady) => EventAction::Used,
            },
            Event::Key(event, false) if event.state == ElementState::Pressed => {
                if let Some(text) = &event.text {
                    self.save_undo_state(Some(EditOp::KeyInput));
                    if self.received_text(cx, text) == Used {
                        EventAction::Edit
                    } else {
                        EventAction::Unused
                    }
                } else {
                    let opt_cmd = cx
                        .config()
                        .shortcuts()
                        .try_match_event(cx.modifiers(), event);
                    if let Some(cmd) = opt_cmd {
                        match self.cmd_action(cx, cmd, Some(event.physical_key)) {
                            Ok(action) => {
                                if matches!(action, EventAction::Cursor) {
                                    self.set_view_offset_from_cursor(cx);
                                }
                                action
                            }
                            Err(NotReady) => EventAction::Used,
                        }
                    } else {
                        EventAction::Unused
                    }
                }
            }
            Event::Ime(ime) => match ime {
                Ime::Enabled => {
                    match self.current {
                        CurrentAction::None => {
                            self.current = CurrentAction::ImeStart;
                            self.set_ime_cursor_area(cx);
                        }
                        CurrentAction::ImeStart | CurrentAction::ImePreedit { .. } => {
                            // already enabled
                        }
                        CurrentAction::Selection => {
                            // Do not interrupt selection
                            cx.cancel_ime_focus(&self.id);
                        }
                    }
                    if !self.has_key_focus {
                        EventAction::FocusGained
                    } else {
                        EventAction::Used
                    }
                }
                Ime::Disabled => {
                    self.clear_ime();
                    if !self.has_key_focus {
                        EventAction::FocusLost
                    } else {
                        EventAction::Used
                    }
                }
                Ime::Preedit { text, cursor } => {
                    self.save_undo_state(None);
                    let mut edit_range = match self.current.clone() {
                        CurrentAction::ImeStart if cursor.is_some() => self.selection.range(),
                        CurrentAction::ImeStart => return EventAction::Used,
                        CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
                        _ => return EventAction::Used,
                    };

                    self.replace_range(edit_range.clone(), text);
                    edit_range.end = edit_range.start + text.len();
                    if let Some((start, end)) = cursor {
                        self.selection.set_sel_index_only(edit_range.start + start);
                        self.selection.set_edit_index(edit_range.start + end);
                    } else {
                        self.selection.set_cursor(edit_range.start + text.len());
                    }

                    self.current = CurrentAction::ImePreedit {
                        edit_range: edit_range.cast(),
                    };
                    self.edit_x_coord = None;
                    EventAction::Preedit
                }
                Ime::Commit { text } => {
                    self.save_undo_state(Some(EditOp::Ime));
                    let edit_range = match self.current.clone() {
                        CurrentAction::ImeStart => self.selection.range(),
                        CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
                        _ => return EventAction::Used,
                    };

                    self.replace_range(edit_range.clone(), text);
                    self.selection.set_cursor(edit_range.start + text.len());

                    self.current = CurrentAction::ImePreedit {
                        edit_range: self.selection.range().cast(),
                    };
                    self.edit_x_coord = None;
                    EventAction::Edit
                }
                Ime::DeleteSurrounding {
                    before_bytes,
                    after_bytes,
                } => {
                    self.save_undo_state(None);
                    let edit_range = match self.current.clone() {
                        CurrentAction::ImeStart => self.selection.range(),
                        CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
                        _ => return EventAction::Used,
                    };

                    if before_bytes > 0 {
                        let end = edit_range.start;
                        let start = end - before_bytes;
                        if self.as_str().is_char_boundary(start) {
                            self.replace_range(start..end, "");
                            self.selection.delete_range(start..end);
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

                    if let Some(text) = self.ime_surrounding_text() {
                        cx.update_ime_surrounding_text(&self.id, text);
                    }

                    EventAction::Used
                }
            },
            Event::PressStart(press) if press.is_tertiary() => {
                match press.grab_click(self.id.clone()).complete(cx) {
                    Unused => EventAction::Unused,
                    Used => EventAction::Used,
                }
            }
            Event::PressEnd { press, .. } if press.is_tertiary() => {
                self.set_cursor_from_coord(cx, press.coord);
                self.cancel_selection_and_ime(cx);
                self.request_key_focus(cx, FocusSource::Pointer);

                if let Some(content) = cx.get_primary() {
                    self.save_undo_state(Some(EditOp::Clipboard));

                    let index = self.selection.edit_index();
                    let range = self.trim_paste(&content);

                    self.replace_range(index..index, &content[range.clone()]);
                    self.selection.set_cursor(index + range.len());
                    self.edit_x_coord = None;
                    EventAction::Edit
                } else {
                    EventAction::Used
                }
            }
            event => match self.input_handler.handle(cx, self.id.clone(), event) {
                TextInputAction::Used => EventAction::Used,
                TextInputAction::Unused => EventAction::Unused,
                TextInputAction::PressStart {
                    coord,
                    clear,
                    repeats,
                } => {
                    if self.current.is_ime_enabled() {
                        self.clear_ime();
                        cx.cancel_ime_focus(&self.id);
                    }
                    self.save_undo_state(Some(EditOp::Cursor));
                    self.current = CurrentAction::Selection;

                    self.set_cursor_from_coord(cx, coord);
                    self.selection.set_anchor(clear);
                    if repeats > 1 {
                        self.selection.expand(
                            self.text.as_str(),
                            &|index| self.display.find_line(index).map(|r| r.1),
                            repeats >= 3,
                        );
                    }

                    self.request_key_focus(cx, FocusSource::Pointer);
                    EventAction::Used
                }
                TextInputAction::PressMove { coord, repeats } => {
                    if self.current == CurrentAction::Selection {
                        self.set_cursor_from_coord(cx, coord);
                        if repeats > 1 {
                            self.selection.expand(
                                self.text.as_str(),
                                &|index| self.display.find_line(index).map(|r| r.1),
                                repeats >= 3,
                            );
                        }
                    }

                    EventAction::Used
                }
                TextInputAction::PressEnd { coord } => {
                    if self.current.is_ime_enabled() {
                        self.clear_ime();
                        cx.cancel_ime_focus(&self.id);
                    }
                    self.save_undo_state(Some(EditOp::Cursor));
                    if self.current == CurrentAction::Selection {
                        self.set_primary(cx);
                    } else {
                        self.set_cursor_from_coord(cx, coord);
                        self.selection.set_empty();
                    }
                    self.current = CurrentAction::None;

                    self.request_key_focus(cx, FocusSource::Pointer);
                    EventAction::Used
                }
            },
        }
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
    fn replace_range(&mut self, range: std::ops::Range<usize>, replace_with: &str) {
        self.text.replace_range(range, replace_with);
        self.require_reprepare();
    }

    /// Cancel on-going selection and IME actions
    ///
    /// This should be called if e.g. key-input interrupts the current
    /// action.
    fn cancel_selection_and_ime(&mut self, cx: &mut EventState) {
        if self.current == CurrentAction::Selection {
            self.input_handler.stop_selecting();
            self.current = CurrentAction::None;
        } else if self.current.is_ime_enabled() {
            self.clear_ime();
            cx.cancel_ime_focus(&self.id);
        }
    }

    /// Clean up IME state
    ///
    /// One should also call [`EventCx::cancel_ime_focus`] unless this is
    /// implied.
    fn clear_ime(&mut self) {
        if self.current.is_ime_enabled() {
            let action = std::mem::replace(&mut self.current, CurrentAction::None);
            if let CurrentAction::ImePreedit { edit_range } = action {
                self.selection.set_cursor(edit_range.start.cast());
                self.replace_range(edit_range.cast(), "");
            }
        }
    }

    fn ime_surrounding_text(&self) -> Option<ImeSurroundingText> {
        const MAX_TEXT_BYTES: usize = ImeSurroundingText::MAX_TEXT_BYTES;

        let sel_range = self.selection.range();
        let edit_range = match self.current.clone() {
            CurrentAction::ImePreedit { edit_range } => Some(edit_range.cast()),
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

        let cursor = self.selection.edit_index().saturating_sub(start);
        // Terminology difference: our sel_index is called 'anchor'
        // SelectionHelper::anchor is not the same thing.
        let sel_index = self.selection.sel_index().saturating_sub(start);
        ImeSurroundingText::new(text, cursor, sel_index)
            .inspect_err(|err| {
                // TODO: use Display for err not Debug
                log::warn!("Editor::ime_surrounding_text failed: {err:?}")
            })
            .ok()
    }

    /// Call to set IME position only while IME is active
    fn set_ime_cursor_area(&self, cx: &mut EventState) {
        if !self.is_prepared() {
            return;
        }

        let range = match self.current.clone() {
            CurrentAction::ImeStart => self.selection.range(),
            CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
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

        cx.set_ime_cursor_area(&self.id, rect + Offset::conv(self.rect.pos));
    }

    /// Call before an edit to (potentially) commit current state based on last_edit
    ///
    /// Call with [`None`] to force commit of any uncommitted changes.
    fn save_undo_state(&mut self, edit: Option<EditOp>) {
        if let Some(op) = edit
            && op.try_merge(&mut self.last_edit)
        {
            return;
        }

        self.last_edit = edit;
        self.undo_stack
            .try_push((self.as_str().to_string(), *self.selection));
    }

    /// Insert `text` at the cursor position
    ///
    /// Committing undo state is the responsibility of the caller.
    fn received_text(&mut self, cx: &mut EventCx, text: &str) -> IsUsed {
        if self.read_only {
            return Unused;
        }
        self.cancel_selection_and_ime(cx);

        let selection = self.selection.range();
        self.replace_range(selection.clone(), text);
        self.selection.set_cursor(selection.start + text.len());
        self.edit_x_coord = None;

        Used
    }

    /// Request key focus, if we don't have it or IME
    fn request_key_focus(&self, cx: &mut EventCx, source: FocusSource) {
        if !self.has_key_focus && !self.current.is_ime_enabled() {
            cx.request_key_focus(self.id.clone(), source);
        }
    }

    fn trim_paste(&self, text: &str) -> Range<usize> {
        let mut end = text.len();
        if !self.wrap {
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

    /// Drive action of a [`Command`]
    fn cmd_action(
        &mut self,
        cx: &mut EventCx,
        mut cmd: Command,
        code: Option<PhysicalKey>,
    ) -> Result<EventAction, NotReady> {
        debug_assert!(self.is_prepared());

        let editable = !self.read_only;
        let mut shift = cx.modifiers().shift_key();
        let mut buf = [0u8; 4];
        let cursor = self.selection.edit_index();
        let len = self.as_str().len();
        let multi_line = self.wrap;
        let selection = self.selection.range();
        let have_sel = selection.end > selection.start;
        let string;

        if self.text_is_rtl() {
            match cmd {
                Command::Left => cmd = Command::Right,
                Command::Right => cmd = Command::Left,
                Command::WordLeft => cmd = Command::WordRight,
                Command::WordRight => cmd = Command::WordLeft,
                _ => (),
            };
        }

        enum Action<'a> {
            None,
            Deselect,
            Activate,
            Insert(&'a str, EditOp),
            Delete(Range<usize>, EditOp),
            Move(usize, Option<f32>),
            UndoRedo(bool),
        }

        let action = match cmd {
            Command::Escape | Command::Deselect if !selection.is_empty() => Action::Deselect,
            Command::Activate => Action::Activate,
            Command::Enter if shift || !multi_line => Action::Activate,
            Command::Enter if editable && multi_line => {
                Action::Insert('\n'.encode_utf8(&mut buf), EditOp::KeyInput)
            }
            // NOTE: we might choose to optionally handle Tab in the future,
            // but without some workaround it prevents keyboard navigation.
            // Command::Tab => Action::Insert('\t'.encode_utf8(&mut buf), EditOp::Insert),
            Command::Left | Command::Home if !shift && have_sel => {
                Action::Move(selection.start, None)
            }
            Command::Left if cursor > 0 => GraphemeCursor::new(cursor, len, true)
                .prev_boundary(self.as_str(), 0)
                .unwrap()
                .map(|index| Action::Move(index, None))
                .unwrap_or(Action::None),
            Command::Right | Command::End if !shift && have_sel => {
                Action::Move(selection.end, None)
            }
            Command::Right if cursor < len => GraphemeCursor::new(cursor, len, true)
                .next_boundary(self.as_str(), 0)
                .unwrap()
                .map(|index| Action::Move(index, None))
                .unwrap_or(Action::None),
            Command::WordLeft if cursor > 0 => {
                let mut iter = self.as_str()[0..cursor].split_word_bound_indices();
                let mut p = iter.next_back().map(|(index, _)| index).unwrap_or(0);
                while self.as_str()[p..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
                {
                    if let Some((index, _)) = iter.next_back() {
                        p = index;
                    } else {
                        break;
                    }
                }
                Action::Move(p, None)
            }
            Command::WordRight if cursor < len => {
                let mut iter = self.as_str()[cursor..].split_word_bound_indices().skip(1);
                let mut p = iter.next().map(|(index, _)| cursor + index).unwrap_or(len);
                while self.as_str()[p..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
                {
                    if let Some((index, _)) = iter.next() {
                        p = cursor + index;
                    } else {
                        break;
                    }
                }
                Action::Move(p, None)
            }
            // Avoid use of unused navigation keys (e.g. by ScrollComponent):
            Command::Left | Command::Right | Command::WordLeft | Command::WordRight => Action::None,
            Command::Up | Command::Down if multi_line => {
                let x = match self.edit_x_coord {
                    Some(x) => x,
                    None => self
                        .display
                        .text_glyph_pos(cursor)
                        .next_back()
                        .map(|r| r.pos.0)
                        .unwrap_or(0.0),
                };
                let mut line = self.display.find_line(cursor).map(|r| r.0).unwrap_or(0);
                // We can tolerate invalid line numbers here!
                line = match cmd {
                    Command::Up => line.wrapping_sub(1),
                    Command::Down => line.wrapping_add(1),
                    _ => unreachable!(),
                };
                const HALF: usize = usize::MAX / 2;
                let nearest_end = match line {
                    0..=HALF => len,
                    _ => 0,
                };
                self.display
                    .line_index_nearest(line, x)
                    .map(|index| Action::Move(index, Some(x)))
                    .unwrap_or(Action::Move(nearest_end, None))
            }
            Command::Home if cursor > 0 => {
                let index = self
                    .display
                    .find_line(cursor)
                    .map(|r| r.1.start)
                    .unwrap_or(0);
                Action::Move(index, None)
            }
            Command::End if cursor < len => {
                let index = self
                    .display
                    .find_line(cursor)
                    .map(|r| r.1.end)
                    .unwrap_or(len);
                Action::Move(index, None)
            }
            Command::DocHome if cursor > 0 => Action::Move(0, None),
            Command::DocEnd if cursor < len => Action::Move(len, None),
            // Avoid use of unused navigation keys (e.g. by ScrollComponent):
            Command::Home | Command::End | Command::DocHome | Command::DocEnd => Action::None,
            Command::PageUp | Command::PageDown if multi_line => {
                let mut v = self
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
                if let Some(line) = self.display.lines().next() {
                    line_height = line.bottom() - line.top();
                }
                let mut h_dist = line_height * 10.0;
                if cmd == Command::PageUp {
                    h_dist *= -1.0;
                }
                v.1 += h_dist;
                Action::Move(self.display.text_index_nearest(v.into()), Some(v.0))
            }
            Command::Delete | Command::DelBack if editable && have_sel => {
                Action::Delete(selection.clone(), EditOp::Delete)
            }
            Command::Delete if editable => GraphemeCursor::new(cursor, len, true)
                .next_boundary(self.as_str(), 0)
                .unwrap()
                .map(|next| Action::Delete(cursor..next, EditOp::Delete))
                .unwrap_or(Action::None),
            Command::DelBack if editable => GraphemeCursor::new(cursor, len, true)
                .prev_boundary(self.as_str(), 0)
                .unwrap()
                .map(|prev| Action::Delete(prev..cursor, EditOp::Delete))
                .unwrap_or(Action::None),
            Command::DelWord if editable => {
                let next = self.as_str()[cursor..]
                    .split_word_bound_indices()
                    .nth(1)
                    .map(|(index, _)| cursor + index)
                    .unwrap_or(len);
                Action::Delete(cursor..next, EditOp::Delete)
            }
            Command::DelWordBack if editable => {
                let prev = self.as_str()[0..cursor]
                    .split_word_bound_indices()
                    .next_back()
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                Action::Delete(prev..cursor, EditOp::Delete)
            }
            Command::SelectAll => {
                self.selection.set_sel_index(0);
                shift = true; // hack
                Action::Move(len, None)
            }
            Command::Cut if editable && have_sel => {
                cx.set_clipboard((self.as_str()[selection.clone()]).into());
                Action::Delete(selection.clone(), EditOp::Clipboard)
            }
            Command::Copy if have_sel => {
                cx.set_clipboard((self.as_str()[selection.clone()]).into());
                Action::None
            }
            Command::Paste if editable => {
                if let Some(content) = cx.get_clipboard() {
                    let range = self.trim_paste(&content);
                    string = content;
                    Action::Insert(&string[range], EditOp::Clipboard)
                } else {
                    Action::None
                }
            }
            Command::Undo | Command::Redo if editable => Action::UndoRedo(cmd == Command::Redo),
            _ => return Ok(EventAction::Unused),
        };

        // We can receive some commands without key focus as a result of
        // selection focus. Request focus on edit actions (like Command::Cut).
        if !matches!(action, Action::None | Action::Deselect) {
            self.request_key_focus(cx, FocusSource::Synthetic);
        }

        if !matches!(action, Action::None) {
            self.cancel_selection_and_ime(cx);
        }

        let edit_op = match action {
            Action::None => return Ok(EventAction::Used),
            Action::Deselect | Action::Move(_, _) => Some(EditOp::Cursor),
            Action::Activate | Action::UndoRedo(_) => None,
            Action::Insert(_, edit) | Action::Delete(_, edit) => Some(edit),
        };
        self.save_undo_state(edit_op);

        let action = match action {
            Action::None => unreachable!(),
            Action::Deselect => {
                self.selection.set_empty();
                cx.redraw();
                EventAction::Cursor
            }
            Action::Activate => EventAction::Activate(code),
            Action::Insert(s, _) => {
                let mut index = cursor;
                let range = if have_sel {
                    index = selection.start;
                    selection.clone()
                } else {
                    index..index
                };
                self.replace_range(range, s);
                self.selection.set_cursor(index + s.len());
                self.edit_x_coord = None;
                EventAction::Edit
            }
            Action::Delete(sel, _) => {
                self.replace_range(sel.clone(), "");
                self.selection.set_cursor(sel.start);
                self.edit_x_coord = None;
                EventAction::Edit
            }
            Action::Move(index, x_coord) => {
                self.selection.set_edit_index(index);
                if !shift {
                    self.selection.set_empty();
                } else {
                    self.set_primary(cx);
                }
                self.edit_x_coord = x_coord;
                cx.redraw();
                EventAction::Cursor
            }
            Action::UndoRedo(redo) => {
                if let Some((text, cursor)) = self.undo_stack.undo_or_redo(redo) {
                    if self.text.as_str() != text {
                        self.text = text.clone();
                        self.status = Status::New;
                        self.edit_x_coord = None;
                    }
                    self.selection = (*cursor).into();
                    EventAction::Edit
                } else {
                    EventAction::Used
                }
            }
        };

        Ok(action)
    }

    /// Set cursor position. It is assumed that the text has not changed.
    ///
    /// Committing undo state is the responsibility of the caller.
    fn set_cursor_from_coord(&mut self, cx: &mut EventCx, coord: Coord) {
        let rel_pos: Vec2 = (coord - self.rect.pos).cast();
        if self.is_prepared() {
            let index = self.display.text_index_nearest(rel_pos.into());
            if index != self.selection.edit_index() {
                self.selection.set_edit_index(index);
                self.set_view_offset_from_cursor(cx);
                self.edit_x_coord = None;
                cx.redraw();
            }
        }
    }

    /// Set primary clipboard (mouse buffer) contents from selection
    fn set_primary(&self, cx: &mut EventCx) {
        if self.has_key_focus && !self.selection.is_empty() && cx.has_primary() {
            let range = self.selection.range();
            cx.set_primary(String::from(&self.as_str()[range]));
        }
    }

    /// Update view_offset after the cursor index changes
    ///
    /// It is assumed that the text has not changed.
    ///
    /// A redraw is assumed since the cursor moved.
    fn set_view_offset_from_cursor(&mut self, cx: &mut EventCx) {
        let cursor = self.selection.edit_index();
        if self.is_prepared()
            && let Some(marker) = self.display.text_glyph_pos(cursor).next_back()
        {
            let y0 = (marker.pos.1 - marker.ascent).cast_floor();
            let pos = self.rect.pos + Offset(marker.pos.0.cast_nearest(), y0);
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
        &self.part.id
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

    /// Commit outstanding changes to the undo history
    ///
    /// Call this *before* changing the text with [`Self::set_str`] or
    /// [`Self::set_string`] to commit changes to the undo history.
    #[inline]
    pub fn pre_commit(&mut self) {
        self.part.save_undo_state(Some(EditOp::Synthetic));
    }

    /// Clear text contents and undo history
    #[inline]
    pub fn clear(&mut self, cx: &mut EventState) {
        self.part.last_edit = Some(EditOp::Initial);
        self.part.undo_stack.clear();
        self.set_string(cx, String::new());
    }

    /// Set text contents from a `str`
    ///
    /// This does not interact with undo history; see also [`Self::clear`],
    /// [`Self::pre_commit`].
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
    /// This does not interact with undo history or call action handlers on the
    /// guard.
    pub fn set_string(&mut self, cx: &mut EventState, text: String) {
        if self.as_str() == text {
            return; // no change
        }

        self.part.cancel_selection_and_ime(cx);

        self.part.text = text;
        self.part.require_reprepare();

        let len = self.as_str().len();
        self.part.selection.set_max_len(len);
        self.part.edit_x_coord = None;
        self.error_state = None;
    }

    /// Replace selected text
    ///
    /// This does not interact with undo history or call action handlers on the
    /// guard.
    #[inline]
    pub fn replace_selected_text(&mut self, cx: &mut EventState, text: &str) {
        self.part.cancel_selection_and_ime(cx);

        let selection = self.part.selection.range();
        self.part.replace_range(selection.clone(), text);
        self.part.selection.set_cursor(selection.start + text.len());
        self.part.edit_x_coord = None;
        self.error_state = None;
    }

    /// Access the cursor index / selection range
    #[inline]
    pub fn cursor_range(&self) -> CursorRange {
        *self.part.selection
    }

    /// Set the cursor index / range
    ///
    /// This does not interact with undo history or call action handlers on the
    /// guard.
    #[inline]
    pub fn set_cursor_range(&mut self, range: CursorRange) {
        self.part.edit_x_coord = None;
        self.part.selection = range.into();
    }

    /// Get whether this text-edit widget is read-only
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.part.read_only
    }

    /// Set whether this text-edit widget is editable
    #[inline]
    pub fn set_read_only(&mut self, read_only: bool) {
        self.part.read_only = read_only;
    }

    /// True if the editor uses multi-line mode
    #[inline]
    pub fn multi_line(&self) -> bool {
        self.part.wrap
    }

    /// Get whether the widget has input focus
    ///
    /// This is true when the widget is has keyboard or IME focus.
    #[inline]
    pub fn has_input_focus(&self) -> bool {
        self.part.has_key_focus || self.part.current.is_ime_enabled()
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
