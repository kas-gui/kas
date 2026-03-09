// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text editor component

use super::highlight::{self, Highlighter, SchemeColors};
use super::*;
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{ElementState, FocusSource, Ime, ImePurpose, ImeSurroundingText, Scroll};
use kas::geom::Vec2;
use kas::prelude::*;
use kas::text::format::FormattableText;
use kas::text::{CursorRange, NotReady, SelectionHelper, format};
use kas::theme::{Background, Text, TextClass};
use kas::util::UndoStack;
use std::borrow::Cow;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

/// Inner editor component
///
/// This type is made public for use as the associated `Target` type of the
/// [`Deref`](std::ops::Deref) impl on `EditField` and `EditBox`. It will no
/// longer be needed once `impl trait` is stabilised for associated types.
/// (Alternatively, [`Editor`] could be re-implemented on the above widgets;
/// this is preferable in theory but requires a lot of tedious code.)
#[autoimpl(Debug where H: trait)]
pub struct EditorComponent<H: Highlighter> {
    // TODO(opt): id, pos are duplicated here since macros don't let us put the core here
    id: Id,
    editable: bool,
    text: Text<highlight::Text<H>>,
    colors: SchemeColors,
    selection: SelectionHelper,
    edit_x_coord: Option<f32>,
    last_edit: Option<EditOp>,
    undo_stack: UndoStack<(String, CursorRange)>,
    has_key_focus: bool,
    current: CurrentAction,
    error_state: bool,
    error_message: Option<Cow<'static, str>>,
    input_handler: TextInput,
}

/// Editor component
///
/// This is a component used to implement an editor widget. It is used, for
/// example, in [`EditField`].
///
/// ### Special behaviour
///
/// This component implements [`Layout`], but only requests the minimum size
/// allocation required to display its current text contents. The wrapping
/// widget may wish to reserve extra space, use a higher stretch policy and
/// potentially also set an alignment hint.
///
/// The wrapping widget may (optionally) wish to implement [`Viewport`] to
/// support scrolling of text content. Since this component is not a widget it
/// cannot implement [`Viewport`] directly, but it does provide the following
/// methods: [`Self::content_size`], [`Self::draw_with_offset`].
#[autoimpl(Debug where H: trait)]
#[autoimpl(Deref, DerefMut using self.0)]
pub struct Component<H: Highlighter>(EditorComponent<H>);

impl<H: Highlighter> Layout for Component<H> {
    #[inline]
    fn rect(&self) -> Rect {
        self.text.rect()
    }

    #[inline]
    fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
        self.text.size_rules(cx, axis)
    }

    fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
        self.text.set_rect(cx, rect, hints);
        self.text.ensure_no_left_overhang();
        if self.current.is_ime_enabled() {
            self.set_ime_cursor_area(cx);
        }
    }

    #[inline]
    fn draw(&self, draw: DrawCx) {
        self.draw_with_offset(draw, self.rect(), Offset::ZERO);
    }
}

impl<H: Default + Highlighter> Default for Component<H> {
    #[inline]
    fn default() -> Self {
        Component(EditorComponent {
            id: Id::default(),
            editable: true,
            text: Text::new(Default::default(), TextClass::Editor, false),
            colors: SchemeColors::default(),
            selection: Default::default(),
            edit_x_coord: None,
            last_edit: Some(EditOp::Initial),
            undo_stack: UndoStack::new(),
            has_key_focus: false,
            current: CurrentAction::None,
            error_state: false,
            error_message: None,
            input_handler: Default::default(),
        })
    }
}

impl<H: Default + Highlighter, S: ToString> From<S> for Component<H> {
    #[inline]
    fn from(text: S) -> Self {
        let text = text.to_string();
        let len = text.len();
        let text = highlight::Text::new(H::default(), text);
        Component(EditorComponent {
            text: Text::new(text, TextClass::Editor, false),
            selection: SelectionHelper::from(len),
            ..Self::default().0
        })
    }
}

impl<H: Highlighter> Component<H> {
    /// Replace the highlighter
    #[inline]
    pub fn with_highlighter<H2: Highlighter>(self, highlighter: H2) -> Component<H2> {
        let class = self.class();
        let wrap = self.multi_line();
        let text = self.0.text.take_text().take_text();
        let text = highlight::Text::new(highlighter, text);

        Component(EditorComponent {
            id: self.0.id,
            editable: self.0.editable,
            text: Text::new(text, class, wrap),
            colors: self.0.colors,
            selection: self.0.selection,
            edit_x_coord: self.0.edit_x_coord,
            last_edit: self.0.last_edit,
            undo_stack: self.0.undo_stack,
            has_key_focus: self.0.has_key_focus,
            current: self.0.current,
            error_state: self.0.error_state,
            error_message: self.0.error_message,
            input_handler: self.0.input_handler,
        })
    }

    /// Set a new highlighter of the same type
    pub fn set_highlighter(&mut self, highlighter: H) {
        self.text.text_mut().set_highlighter(highlighter);
    }

    /// Get the background color
    pub fn background_color(&self) -> Background {
        if self.error_state {
            Background::Error
        } else if let Some(c) = self.colors.background.as_rgba() {
            Background::Rgb(c.as_rgb())
        } else {
            Background::Default
        }
    }

    /// Access text
    #[inline]
    pub fn text(&self) -> &Text<impl FormattableText> {
        &self.text
    }

    /// Access text (mut)
    ///
    /// It is left to the wrapping widget to ensure this is not mis-used.
    #[inline]
    pub fn text_mut(&mut self) -> &mut Text<impl FormattableText> {
        &mut self.text
    }

    /// Set the initial text (inline)
    ///
    /// This method should only be used on a new `Editor`.
    #[inline]
    #[must_use]
    pub fn with_text(mut self, text: impl ToString) -> Self {
        debug_assert!(self.current == CurrentAction::None && !self.input_handler.is_selecting());
        let text = text.to_string();
        let len = text.len();
        self.text.text_mut().set_text(text);
        self.selection.set_cursor(len);
        self
    }

    /// Configure component
    #[inline]
    pub fn configure(&mut self, cx: &mut ConfigCx, id: Id) {
        self.id = id;
        self.text.text_mut().configure(cx);
        self.0.colors = self.text.text().scheme_colors();
        self.text.configure(&mut cx.size_cx());
    }

    /// Implementation of [`Viewport::content_size`]
    pub fn content_size(&self) -> Size {
        if let Ok((tl, br)) = self.text.bounding_box() {
            (br - tl).cast_ceil()
        } else {
            Size::ZERO
        }
    }

    /// Implementation of [`Viewport::draw_with_offset`]
    pub fn draw_with_offset(&self, mut draw: DrawCx, rect: Rect, offset: Offset) {
        let Ok(display) = self.text.display() else {
            return;
        };

        let pos = self.rect().pos - offset;
        let range: Range<u32> = self.selection.range().cast();

        let color_tokens = self.text.color_tokens();
        let default_colors = format::Colors {
            color: self.colors.foreground,
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
            buf[1].1.color = self.colors.selection_foreground;
            buf[1].1.background = Some(self.colors.selection_background);
            buf[2].0 = range.end;
            let r0 = if range.start > 0 { 0 } else { 1 };
            &buf[r0..]
        } else {
            let set_selection_colors = |colors: &mut format::Colors| {
                if colors.color == self.colors.foreground {
                    colors.color = self.colors.selection_foreground;
                }
                colors.background = Some(self.colors.selection_background);
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
        draw.text_with_colors(pos, rect, display, tokens);

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
            draw.decorate_text(pos, rect, display, &tokens[r0..]);
        }

        if self.editable && draw.ev_state().has_input_focus(self.id_ref()) == Some(true) {
            draw.text_cursor(
                pos,
                rect,
                &self.text,
                self.selection.edit_index(),
                Some(self.colors.cursor),
            );
        }
    }

    /// Handle an event
    pub fn handle_event(&mut self, cx: &mut EventCx, event: Event) -> EventAction {
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
                Ok(action) => action,
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
                            Ok(action) => action,
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
                            cx.cancel_ime_focus(self.id_ref());
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

                    self.text.replace_range(edit_range.clone(), text);
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
                    self.prepare_and_scroll(cx, false);
                    EventAction::Used
                }
                Ime::Commit { text } => {
                    self.save_undo_state(Some(EditOp::Ime));
                    let edit_range = match self.current.clone() {
                        CurrentAction::ImeStart => self.selection.range(),
                        CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
                        _ => return EventAction::Used,
                    };

                    self.text.replace_range(edit_range.clone(), text);
                    self.selection.set_cursor(edit_range.start + text.len());

                    self.current = CurrentAction::ImePreedit {
                        edit_range: self.selection.range().cast(),
                    };
                    self.edit_x_coord = None;
                    self.prepare_and_scroll(cx, false);
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
                            self.text.replace_range(start..end, "");
                            self.selection.delete_range(start..end);
                        } else {
                            log::warn!("buggy IME tried to delete range not at char boundary");
                        }
                    }

                    if after_bytes > 0 {
                        let start = edit_range.end;
                        let end = start + after_bytes;
                        if self.as_str().is_char_boundary(end) {
                            self.text.replace_range(start..end, "");
                        } else {
                            log::warn!("buggy IME tried to delete range not at char boundary");
                        }
                    }

                    if let Some(text) = self.ime_surrounding_text() {
                        cx.update_ime_surrounding_text(self.id_ref(), text);
                    }

                    EventAction::Used
                }
            },
            Event::PressStart(press) if press.is_tertiary() => {
                match press.grab_click(self.id()).complete(cx) {
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

                    self.text
                        .replace_range(index..index, &content[range.clone()]);
                    self.selection.set_cursor(index + range.len());
                    self.edit_x_coord = None;
                    self.prepare_and_scroll(cx, false);

                    EventAction::Edit
                } else {
                    EventAction::Used
                }
            }
            event => match self.0.input_handler.handle(cx, self.0.id.clone(), event) {
                TextInputAction::Used => EventAction::Used,
                TextInputAction::Unused => EventAction::Unused,
                TextInputAction::PressStart {
                    coord,
                    clear,
                    repeats,
                } => {
                    if self.current.is_ime_enabled() {
                        self.clear_ime();
                        cx.cancel_ime_focus(self.id_ref());
                    }
                    self.save_undo_state(Some(EditOp::Cursor));
                    self.current = CurrentAction::Selection;

                    self.set_cursor_from_coord(cx, coord);
                    self.selection.set_anchor(clear);
                    if repeats > 1 {
                        self.0.selection.expand(&self.0.text, repeats >= 3);
                    }

                    self.request_key_focus(cx, FocusSource::Pointer);
                    EventAction::Used
                }
                TextInputAction::PressMove { coord, repeats } => {
                    if self.current == CurrentAction::Selection {
                        self.set_cursor_from_coord(cx, coord);
                        if repeats > 1 {
                            self.0.selection.expand(&self.0.text, repeats >= 3);
                        }
                    }

                    EventAction::Used
                }
                TextInputAction::PressEnd { coord } => {
                    if self.current.is_ime_enabled() {
                        self.clear_ime();
                        cx.cancel_ime_focus(self.id_ref());
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
}

impl<H: Highlighter> EditorComponent<H> {
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
                self.text.replace_range(edit_range.cast(), "");
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

        if let Ok(Some((_, line_range))) = self.text.find_line(range.start) {
            range.start = line_range.start;
        }
        if let Ok(Some((_, line_range))) = self.text.find_line(range.end) {
            range.end = line_range.end;
        }

        if range.len() - edit_len > MAX_TEXT_BYTES {
            range.end = range.end.min(initial_range.end + MAX_TEXT_BYTES / 2);
            while !self.text.as_str().is_char_boundary(range.end) {
                range.end -= 1;
            }

            if range.len() - edit_len > MAX_TEXT_BYTES {
                range.start = range.start.max(initial_range.start - MAX_TEXT_BYTES / 2);
                while !self.text.as_str().is_char_boundary(range.start) {
                    range.start += 1;
                }
            }
        }

        let start = range.start;
        let mut text = String::with_capacity(range.len() - edit_len);
        if let Some(er) = edit_range {
            text.push_str(&self.text.as_str()[range.start..er.start]);
            text.push_str(&self.text.as_str()[er.end..range.end]);
        } else {
            text = self.text.as_str()[range].to_string();
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
        if let Ok(text) = self.text.display() {
            let range = match self.current.clone() {
                CurrentAction::ImeStart => self.selection.range(),
                CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
                _ => return,
            };

            let (m1, m2);
            if range.is_empty() {
                let mut iter = text.text_glyph_pos(range.start);
                m1 = iter.next();
                m2 = iter.next();
            } else {
                m1 = text.text_glyph_pos(range.start).next_back();
                m2 = text.text_glyph_pos(range.end).next();
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

            cx.set_ime_cursor_area(&self.id, rect + Offset::conv(self.text.rect().pos));
        }
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
            .try_push((self.clone_string(), self.cursor_range()));
    }

    /// Prepare text
    ///
    /// Updates the view offset (scroll position) if the content size changes or
    /// `force_set_offset`. Requests redraw and resize as appropriate.
    fn prepare_and_scroll(&mut self, cx: &mut EventCx, force_set_offset: bool) {
        let bb = self.text.bounding_box();
        if self.text.prepare() {
            self.text.ensure_no_left_overhang();
            cx.redraw();
        }

        let mut set_offset = force_set_offset;
        if bb != self.text.bounding_box() {
            cx.resize();
            set_offset = true;
        }
        if set_offset {
            self.set_view_offset_from_cursor(cx);
        }
    }

    /// Insert `text` at the cursor position
    ///
    /// Committing undo state is the responsibility of the caller.
    fn received_text(&mut self, cx: &mut EventCx, text: &str) -> IsUsed {
        if !self.editable {
            return Unused;
        }
        self.cancel_selection_and_ime(cx);

        let index = self.selection.edit_index();
        let selection = self.selection.range();
        let have_sel = selection.start < selection.end;
        if have_sel {
            self.text.replace_range(selection.clone(), text);
            self.selection.set_cursor(selection.start + text.len());
        } else {
            self.text.insert_str(index, text);
            self.selection.set_cursor(index + text.len());
        }
        self.edit_x_coord = None;

        self.prepare_and_scroll(cx, false);
        Used
    }

    /// Request key focus, if we don't have it or IME
    fn request_key_focus(&self, cx: &mut EventCx, source: FocusSource) {
        if !self.has_key_focus && !self.current.is_ime_enabled() {
            cx.request_key_focus(self.id(), source);
        }
    }

    fn trim_paste(&self, text: &str) -> Range<usize> {
        let mut end = text.len();
        if !self.multi_line() {
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
        let editable = self.editable;
        let mut shift = cx.modifiers().shift_key();
        let mut buf = [0u8; 4];
        let cursor = self.selection.edit_index();
        let len = self.text.str_len();
        let multi_line = self.multi_line();
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
                .prev_boundary(self.text.as_str(), 0)
                .unwrap()
                .map(|index| Action::Move(index, None))
                .unwrap_or(Action::None),
            Command::Right | Command::End if !shift && have_sel => {
                Action::Move(selection.end, None)
            }
            Command::Right if cursor < len => GraphemeCursor::new(cursor, len, true)
                .next_boundary(self.text.as_str(), 0)
                .unwrap()
                .map(|index| Action::Move(index, None))
                .unwrap_or(Action::None),
            Command::WordLeft if cursor > 0 => {
                let mut iter = self.text.as_str()[0..cursor].split_word_bound_indices();
                let mut p = iter.next_back().map(|(index, _)| index).unwrap_or(0);
                while self.text.as_str()[p..]
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
                let mut iter = self.text.as_str()[cursor..]
                    .split_word_bound_indices()
                    .skip(1);
                let mut p = iter.next().map(|(index, _)| cursor + index).unwrap_or(len);
                while self.text.as_str()[p..]
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
                        .text
                        .text_glyph_pos(cursor)?
                        .next_back()
                        .map(|r| r.pos.0)
                        .unwrap_or(0.0),
                };
                let mut line = self.text.find_line(cursor)?.map(|r| r.0).unwrap_or(0);
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
                self.text
                    .line_index_nearest(line, x)?
                    .map(|index| Action::Move(index, Some(x)))
                    .unwrap_or(Action::Move(nearest_end, None))
            }
            Command::Home if cursor > 0 => {
                let index = self.text.find_line(cursor)?.map(|r| r.1.start).unwrap_or(0);
                Action::Move(index, None)
            }
            Command::End if cursor < len => {
                let index = self.text.find_line(cursor)?.map(|r| r.1.end).unwrap_or(len);
                Action::Move(index, None)
            }
            Command::DocHome if cursor > 0 => Action::Move(0, None),
            Command::DocEnd if cursor < len => Action::Move(len, None),
            // Avoid use of unused navigation keys (e.g. by ScrollComponent):
            Command::Home | Command::End | Command::DocHome | Command::DocEnd => Action::None,
            Command::PageUp | Command::PageDown if multi_line => {
                let mut v = self
                    .text
                    .text_glyph_pos(cursor)?
                    .next_back()
                    .map(|r| r.pos.into())
                    .unwrap_or(Vec2::ZERO);
                if let Some(x) = self.edit_x_coord {
                    v.0 = x;
                }
                const FACTOR: f32 = 2.0 / 3.0;
                let mut h_dist = f32::conv(self.text.rect().size.1) * FACTOR;
                if cmd == Command::PageUp {
                    h_dist *= -1.0;
                }
                v.1 += h_dist;
                Action::Move(self.text.text_index_nearest(v)?, Some(v.0))
            }
            Command::Delete | Command::DelBack if editable && have_sel => {
                Action::Delete(selection.clone(), EditOp::Delete)
            }
            Command::Delete if editable => GraphemeCursor::new(cursor, len, true)
                .next_boundary(self.text.as_str(), 0)
                .unwrap()
                .map(|next| Action::Delete(cursor..next, EditOp::Delete))
                .unwrap_or(Action::None),
            Command::DelBack if editable => GraphemeCursor::new(cursor, len, true)
                .prev_boundary(self.text.as_str(), 0)
                .unwrap()
                .map(|prev| Action::Delete(prev..cursor, EditOp::Delete))
                .unwrap_or(Action::None),
            Command::DelWord if editable => {
                let next = self.text.as_str()[cursor..]
                    .split_word_bound_indices()
                    .nth(1)
                    .map(|(index, _)| cursor + index)
                    .unwrap_or(len);
                Action::Delete(cursor..next, EditOp::Delete)
            }
            Command::DelWordBack if editable => {
                let prev = self.text.as_str()[0..cursor]
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
                cx.set_clipboard((self.text.as_str()[selection.clone()]).into());
                Action::Delete(selection.clone(), EditOp::Clipboard)
            }
            Command::Copy if have_sel => {
                cx.set_clipboard((self.text.as_str()[selection.clone()]).into());
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
                self.text.replace_range(range, s);
                self.selection.set_cursor(index + s.len());
                self.edit_x_coord = None;
                EventAction::Edit
            }
            Action::Delete(sel, _) => {
                self.text.replace_range(sel.clone(), "");
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
                    if self.text.set_str(text) {
                        self.edit_x_coord = None;
                    }
                    self.selection = (*cursor).into();
                    EventAction::Edit
                } else {
                    EventAction::Used
                }
            }
        };

        self.prepare_and_scroll(cx, true);
        Ok(action)
    }

    /// Set cursor position. It is assumed that the text has not changed.
    ///
    /// Committing undo state is the responsibility of the caller.
    fn set_cursor_from_coord(&mut self, cx: &mut EventCx, coord: Coord) {
        let rel_pos = (coord - self.text.rect().pos).cast();
        if let Ok(index) = self.text.text_index_nearest(rel_pos) {
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
            cx.set_primary(String::from(&self.text.as_str()[range]));
        }
    }

    /// Update view_offset after the cursor index changes
    ///
    /// It is assumed that the text has not changed.
    ///
    /// A redraw is assumed since the cursor moved.
    fn set_view_offset_from_cursor(&mut self, cx: &mut EventCx) {
        let cursor = self.selection.edit_index();
        if let Some(marker) = self
            .text
            .text_glyph_pos(cursor)
            .ok()
            .and_then(|mut m| m.next_back())
        {
            let y0 = (marker.pos.1 - marker.ascent).cast_floor();
            let pos = self.text.rect().pos + Offset(marker.pos.0.cast_nearest(), y0);
            let size = Size(0, i32::conv_ceil(marker.pos.1 - marker.descent) - y0);
            cx.set_scroll(Scroll::Rect(Rect { pos, size }));
        }
    }
}

/// Text editor interface
#[kas::split_impl(for<H: Highlighter> EditorComponent<H>)]
pub trait Editor {
    /// Get a reference to the widget's identifier
    #[inline]
    fn id_ref(&self) -> &Id {
        &self.id
    }

    /// Get the widget's identifier
    #[inline]
    fn id(&self) -> Id {
        self.id.clone()
    }

    /// Get text contents
    #[inline]
    fn as_str(&self) -> &str {
        self.text.as_str()
    }

    /// Get the text contents as a `String`
    #[inline]
    fn clone_string(&self) -> String {
        self.text.as_str().to_string()
    }

    /// Get the (horizontal) text direction
    ///
    /// This returns `true` if the text is inferred to have right-to-left;
    /// in other cases (including when the text is empty) it returns `false`.
    /// TODO: support defaulting to RTL.
    #[inline]
    fn text_is_rtl(&self) -> bool {
        self.text.text_is_rtl()
    }

    /// Commit outstanding changes to the undo history
    ///
    /// Call this *before* changing the text with [`Self::set_str`] or
    /// [`Self::set_string`] to commit changes to the undo history.
    #[inline]
    fn pre_commit(&mut self) {
        self.save_undo_state(Some(EditOp::Synthetic));
    }

    /// Clear text contents and undo history
    ///
    /// This method does not call any [`EditGuard`] actions; consider also
    /// calling [`EditField::call_guard_edit`].
    #[inline]
    fn clear(&mut self, cx: &mut EventState) {
        self.last_edit = Some(EditOp::Initial);
        self.undo_stack.clear();
        self.set_string(cx, String::new());
    }

    /// Set text contents from a `str`
    ///
    /// This does not interact with undo history; see also [`Self::clear`],
    /// [`Self::pre_commit`].
    ///
    /// This method does not call any [`EditGuard`] actions; consider also
    /// calling [`EditField::call_guard_edit`].
    ///
    /// Returns `true` if the text may have changed.
    #[inline]
    fn set_str(&mut self, cx: &mut EventState, text: &str) -> bool {
        if self.text.as_str() != text {
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
    ///
    /// This method clears the error state but does not call any [`EditGuard`]
    /// actions; consider also calling [`EditField::call_guard_edit`].
    ///
    /// Returns `true` if the text is ready and may have changed.
    fn set_string(&mut self, cx: &mut EventState, string: String) -> bool {
        self.cancel_selection_and_ime(cx);

        if !self.text.set_str(&string) {
            return false;
        }

        cx.redraw(self.id());
        let len = self.text.str_len();
        self.selection.set_max_len(len);
        self.edit_x_coord = None;
        self.clear_error();
        self.text.prepare()
    }

    /// Replace selected text
    ///
    /// This does not interact with undo history or call action handlers on the
    /// guard.
    ///
    /// This method clears the error state but does not call any [`EditGuard`]
    /// actions; consider also calling [`EditField::call_guard_edit`].
    ///
    /// Returns `true` if the text is ready and may have changed.
    #[inline]
    fn replace_selected_text(&mut self, cx: &mut EventState, text: &str) -> bool {
        self.cancel_selection_and_ime(cx);

        let index = self.selection.edit_index();
        let selection = self.selection.range();
        let have_sel = selection.start < selection.end;
        if have_sel {
            self.text.replace_range(selection.clone(), text);
            self.selection.set_cursor(selection.start + text.len());
        } else {
            self.text.insert_str(index, text);
            self.selection.set_cursor(index + text.len());
        }
        self.edit_x_coord = None;
        self.clear_error();
        self.text.prepare()
    }

    /// Access the cursor index / selection range
    #[inline]
    fn cursor_range(&self) -> CursorRange {
        *self.selection
    }

    /// Set the cursor index / range
    ///
    /// This does not interact with undo history or call action handlers on the
    /// guard.
    #[inline]
    fn set_cursor_range(&mut self, range: CursorRange) {
        self.edit_x_coord = None;
        self.selection = range.into();
    }

    /// Get whether this `EditField` is editable
    #[inline]
    fn is_editable(&self) -> bool {
        self.editable
    }

    /// Set whether this `EditField` is editable
    #[inline]
    fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }

    /// True if the editor uses multi-line mode
    #[inline]
    fn multi_line(&self) -> bool {
        self.text.wrap()
    }

    /// Get the text class used
    #[inline]
    fn class(&self) -> TextClass {
        self.text.class()
    }

    /// Get whether the widget has input focus
    ///
    /// This is true when the widget is has keyboard or IME focus.
    #[inline]
    fn has_input_focus(&self) -> bool {
        self.has_key_focus || self.current.is_ime_enabled()
    }

    /// Get whether the input state is erroneous
    #[inline]
    fn has_error(&self) -> bool {
        self.error_state
    }

    /// Get the error message, if any
    #[inline]
    fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Clear the error state
    fn clear_error(&mut self) {
        self.error_state = false;
        self.error_message = None;
    }

    /// Mark the input as erroneous with an optional message
    ///
    /// This state should be set from [`EditGuard::edit`] when appropriate. The
    /// state is cleared immediately before calling [`EditGuard::edit`] and also
    /// in case a text is directly assigned (e.g. using [`Self::set_string`]).
    ///
    /// When set, the input field's background is drawn red. If a message is
    /// supplied, then a tooltip will be available on mouse-hover.
    fn set_error(&mut self, cx: &mut EventState, message: Option<Cow<'static, str>>) {
        self.error_state = true;
        self.error_message = message;
        cx.redraw(&self.id);
    }
}
