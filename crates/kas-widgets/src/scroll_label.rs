// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scrollable and selectable label

use super::{ScrollBar, ScrollBarMsg};
use kas::event::components::{ScrollComponent, TextInput, TextInputAction};
use kas::event::{CursorIcon, FocusSource, Scroll};
use kas::prelude::*;
use kas::text::SelectionHelper;
use kas::text::format::FormattableText;
use kas::theme::{Text, TextClass};

#[impl_self]
mod SelectableText {
    /// A text label supporting selection
    ///
    /// The [`ScrollText`] widget should be preferred in most cases; this widget
    /// is a component of `ScrollText` and has some special behaviour.
    ///
    /// Line-wrapping is enabled; default alignment is derived from the script
    /// (usually top-left).
    ///
    /// ### Special behaviour
    ///
    /// This is a [`Viewport`] widget.
    #[widget]
    #[layout(self.text)]
    pub struct SelectableText<A, T: FormattableText + 'static> {
        core: widget_core!(),
        text: Text<T>,
        text_fn: Option<Box<dyn Fn(&ConfigCx, &A) -> T>>,
        selection: SelectionHelper,
        has_sel_focus: bool,
        input_handler: TextInput,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = kas::MacroDefinedLayout::size_rules(self, cx, axis);
            if axis.is_vertical()
                && let Some(width) = axis.other()
            {
                let height = self
                    .text
                    .measure_height(width.cast(), std::num::NonZero::new(3));
                rules.reduce_min_to(height.cast_ceil());
            }
            rules
        }
    }

    impl Viewport for Self {
        #[inline]
        fn content_size(&self) -> Size {
            if let Ok((tl, br)) = self.text.bounding_box() {
                (br - tl).cast_ceil()
            } else {
                Size::ZERO
            }
        }

        fn draw_with_offset(&self, mut draw: DrawCx, rect: Rect, offset: Offset) {
            let pos = self.rect().pos - offset;

            if self.selection.is_empty() {
                draw.text_pos(pos, rect, &self.text);
            } else {
                draw.text_selected(pos, rect, &self.text, self.selection.range());
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::TextLabel {
                text: self.text.as_str(),
                cursor: self.selection.edit_index(),
                sel_index: self.selection.sel_index(),
            }
        }
    }

    impl<T: FormattableText + 'static> SelectableText<(), T> {
        /// Construct a `SelectableText` with the given inital `text`
        ///
        /// The text is set from input data on update.
        #[inline]
        pub fn new(text: T) -> Self {
            SelectableText {
                core: Default::default(),
                text: Text::new(text, TextClass::Standard, true),
                text_fn: None,
                selection: SelectionHelper::new(0, 0),
                has_sel_focus: false,
                input_handler: Default::default(),
            }
        }

        /// Set or replace the text derivation function
        ///
        /// The text is set from input data on update.
        #[inline]
        pub fn with_fn<A>(
            self,
            text_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
        ) -> SelectableText<A, T> {
            SelectableText {
                core: self.core,
                text: self.text,
                text_fn: Some(Box::new(text_fn)),
                selection: self.selection,
                has_sel_focus: self.has_sel_focus,
                input_handler: self.input_handler,
            }
        }
    }

    impl Self {
        /// Construct an `SelectableText` with the given text derivation function
        ///
        /// The text is set from input data on update.
        #[inline]
        pub fn new_fn(text_fn: impl Fn(&ConfigCx, &A) -> T + 'static) -> Self
        where
            T: Default,
        {
            SelectableText::<(), T>::new(T::default()).with_fn(text_fn)
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        ///
        /// Returns `true` when the content size may have changed.
        pub fn set_text(&mut self, text: T) -> bool {
            self.text.set_text(text);
            if !self.text.prepare() {
                return false;
            }

            self.selection.set_max_len(self.text.str_len());
            true
        }

        fn set_cursor_from_coord(&mut self, cx: &mut EventCx, coord: Coord) {
            let rel_pos = (coord - self.rect().pos).cast();
            if let Ok(index) = self.text.text_index_nearest(rel_pos) {
                if index != self.selection.edit_index() {
                    self.selection.set_edit_index(index);
                    self.set_view_offset_from_cursor(cx, index);
                    cx.redraw();
                }
            }
        }

        fn set_primary(&self, cx: &mut EventCx) {
            if self.has_sel_focus && !self.selection.is_empty() && cx.has_primary() {
                let range = self.selection.range();
                cx.set_primary(String::from(&self.text.as_str()[range]));
            }
        }

        /// Update view_offset from `cursor`
        ///
        /// This method is mostly identical to its counterpart in `EditField`.
        fn set_view_offset_from_cursor(&mut self, cx: &mut EventCx, cursor: usize) {
            if let Some(marker) = self
                .text
                .text_glyph_pos(cursor)
                .ok()
                .and_then(|mut m| m.next_back())
            {
                let y0 = (marker.pos.1 - marker.ascent).cast_floor();
                let pos = Coord(marker.pos.0.cast_nearest(), y0);
                let size = Size(0, i32::conv_ceil(marker.pos.1 - marker.descent) - y0);
                cx.set_scroll(Scroll::Rect(Rect { pos, size }));
            }
        }

        /// Get text contents
        #[inline]
        pub fn as_str(&self) -> &str {
            self.text.as_str()
        }
    }

    impl SelectableText<(), String> {
        /// Set text contents from a string
        ///
        /// Returns `true` when the content size may have changed.
        #[inline]
        pub fn set_string(&mut self, string: String) -> bool {
            if self.text.set_string(string) {
                self.text.prepare();
                true
            } else {
                false
            }
        }
    }

    impl Events for Self {
        type Data = A;

        #[inline]
        fn mouse_over_icon(&self) -> Option<CursorIcon> {
            Some(CursorIcon::Text)
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.text);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            if let Some(method) = self.text_fn.as_ref() {
                if self.set_text(method(cx, data)) {
                    cx.resize();
                }
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, _) => match cmd {
                    Command::Escape | Command::Deselect if !self.selection.is_empty() => {
                        self.selection.set_empty();
                        cx.redraw();
                        Used
                    }
                    Command::SelectAll => {
                        self.selection.set_sel_index(0);
                        self.selection.set_edit_index(self.text.str_len());
                        self.set_primary(cx);
                        cx.redraw();
                        Used
                    }
                    Command::Cut | Command::Copy => {
                        let range = self.selection.range();
                        cx.set_clipboard((self.text.as_str()[range]).to_string());
                        Used
                    }
                    _ => Unused,
                },
                Event::SelFocus(source) => {
                    self.has_sel_focus = true;
                    if source == FocusSource::Pointer {
                        self.set_primary(cx);
                    }
                    Used
                }
                Event::LostSelFocus => {
                    self.has_sel_focus = false;
                    self.selection.set_empty();
                    cx.redraw();
                    Used
                }
                event => match self.input_handler.handle(cx, self.id(), event) {
                    TextInputAction::Used => Used,
                    TextInputAction::Unused => Unused,
                    TextInputAction::PressStart {
                        coord,
                        clear,
                        repeats,
                    } => {
                        self.set_cursor_from_coord(cx, coord);
                        self.selection.set_anchor(clear);
                        if repeats > 1 {
                            self.selection.expand(&self.text, repeats >= 3);
                        }

                        if !self.has_sel_focus {
                            cx.request_sel_focus(self.id(), FocusSource::Pointer);
                        }
                        Used
                    }
                    TextInputAction::PressMove { coord, repeats } => {
                        self.set_cursor_from_coord(cx, coord);
                        if repeats > 1 {
                            self.selection.expand(&self.text, repeats >= 3);
                        }
                        Used
                    }
                    TextInputAction::PressEnd { .. } => {
                        if self.has_sel_focus {
                            self.set_primary(cx);
                        }
                        Used
                    }
                },
            }
        }
    }
}

/// A text label supporting selection
///
/// Line-wrapping is enabled; default alignment is derived from the script
/// (usually top-left).
pub type SelectableLabel<T> = SelectableText<(), T>;

#[impl_self]
mod ScrollText {
    /// A text label supporting scrolling and selection
    ///
    /// This widget is a wrapper around [`SelectableText`] enabling scrolling
    /// and adding a vertical scroll bar.
    ///
    /// Line-wrapping is enabled; default alignment is derived from the script
    /// (usually top-left).
    ///
    /// ### Messages
    ///
    /// [`kas::messages::SetScrollOffset`] may be used to set the scroll offset.
    #[widget]
    pub struct ScrollText<A, T: FormattableText + 'static> {
        core: widget_core!(),
        scroll: ScrollComponent,
        // NOTE: label is a Viewport which doesn't use update methods, therefore we don't call them.
        #[widget]
        label: SelectableText<A, T>,
        #[widget = &()]
        vert_bar: ScrollBar<kas::dir::Down>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = self.label.size_rules(cx, axis);
            let _ = self.vert_bar.size_rules(cx, axis);
            if axis.is_vertical() {
                let dpem = cx.dpem(self.label.text.class());
                rules.reduce_min_to((dpem * 4.0).cast_ceil());
            }
            rules.with_stretch(Stretch::Low)
        }

        fn set_rect(&mut self, cx: &mut SizeCx, mut rect: Rect, hints: AlignHints) {
            self.core.set_rect(rect);
            self.label.set_rect(cx, rect, hints);

            let w = cx.scroll_bar_width().min(rect.size.0);
            rect.pos.0 += rect.size.0 - w;
            rect.size.0 = w;
            self.vert_bar.set_rect(cx, rect, AlignHints::NONE);

            self.update_content_size(cx);
        }

        fn draw(&self, mut draw: DrawCx) {
            self.label
                .draw_with_offset(draw.re(), self.rect(), self.scroll.offset());

            // We use a new pass to draw the scroll bar over inner content, but
            // only when required to minimize cost:
            if self.vert_bar.currently_visible(draw.ev_state()) {
                draw.with_pass(|draw| self.vert_bar.draw(draw));
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::ScrollRegion {
                offset: self.scroll.offset(),
                max_offset: self.scroll.max_offset(),
            }
        }

        fn translation(&self, index: usize) -> Offset {
            if index == widget_index!(self.label) {
                self.scroll.offset()
            } else {
                Offset::ZERO
            }
        }
    }

    impl<T: FormattableText + 'static> ScrollText<(), T> {
        /// Construct an `ScrollText` with the given inital `text`
        ///
        /// The text is set from input data on update.
        #[inline]
        pub fn new(text: T) -> Self {
            ScrollText {
                core: Default::default(),
                scroll: Default::default(),
                label: SelectableText::new(text),
                vert_bar: ScrollBar::new().with_invisible(true),
            }
        }

        /// Set or replace the text derivation function
        ///
        /// The text is set from input data on update.
        #[inline]
        pub fn with_fn<A>(
            self,
            text_fn: impl Fn(&ConfigCx, &A) -> T + 'static,
        ) -> ScrollText<A, T> {
            ScrollText {
                core: self.core,
                scroll: self.scroll,
                label: self.label.with_fn(text_fn),
                vert_bar: self.vert_bar,
            }
        }
    }

    impl Self {
        /// Construct an `ScrollText` with the given text derivation function
        ///
        /// The text is set from input data on update.
        #[inline]
        pub fn new_fn(text_fn: impl Fn(&ConfigCx, &A) -> T + 'static) -> Self
        where
            T: Default,
        {
            ScrollText::<(), T>::new(T::default()).with_fn(text_fn)
        }

        /// Replace text
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, cx: &mut EventState, text: T) {
            if self.label.set_text(text) {
                self.update_content_size(cx);
                cx.redraw(self);
            }
        }

        /// Get text contents
        pub fn as_str(&self) -> &str {
            self.label.as_str()
        }

        fn update_content_size(&mut self, cx: &mut EventState) {
            let size = self.rect().size;
            let _ = self.scroll.set_sizes(size, self.label.content_size());
            self.vert_bar
                .set_limits(cx, self.scroll.max_offset().1, size.1);
            self.vert_bar.set_value(cx, self.scroll.offset().1);
        }
    }

    impl ScrollText<(), String> {
        /// Set text contents from a string
        pub fn set_string(&mut self, cx: &mut EventState, string: String) {
            if self.label.set_string(string) {
                self.update_content_size(cx);
            }
        }
    }

    impl Events for Self {
        type Data = A;

        fn probe(&self, coord: Coord) -> Id {
            self.vert_bar
                .try_probe(coord)
                .unwrap_or_else(|| self.label.id())
        }

        #[inline]
        fn mouse_over_icon(&self) -> Option<CursorIcon> {
            Some(CursorIcon::Text)
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            let is_used = self
                .scroll
                .scroll_by_event(cx, event, self.id(), self.rect());
            self.vert_bar.set_value(cx, self.scroll.offset().1);
            is_used
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            let action = if cx.last_child() == Some(widget_index![self.vert_bar])
                && let Some(ScrollBarMsg(y)) = cx.try_pop()
            {
                let offset = Offset(self.scroll.offset().0, y);
                self.scroll.set_offset(offset)
            } else if let Some(kas::messages::SetScrollOffset(offset)) = cx.try_pop() {
                self.scroll.set_offset(offset)
            } else {
                return;
            };

            if action.0 {
                cx.action_moved(action);
                self.vert_bar.set_value(cx, self.scroll.offset().1);
            }
        }

        fn handle_resize(&mut self, cx: &mut ConfigCx, _: &Self::Data) -> ActionResize {
            let size = self.label.rect().size;
            let axis = AxisInfo::new(false, Some(size.1));
            let mut resize = self.label.size_rules(&mut cx.size_cx(), axis).min_size() > size.0;
            let axis = AxisInfo::new(true, Some(size.0));
            resize |= self.label.size_rules(&mut cx.size_cx(), axis).min_size() > size.1;
            self.label
                .set_rect(&mut cx.size_cx(), self.label.rect(), Default::default());
            self.update_content_size(cx);
            ActionResize(resize)
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, _: &Self::Data, scroll: Scroll) {
            self.scroll.scroll(cx, self.id(), self.rect(), scroll);
            self.vert_bar.set_value(cx, self.scroll.offset().1);
        }
    }
}

/// A text label supporting scrolling and selection
///
/// This widget is a wrapper around [`SelectableText`] enabling scrolling
/// and adding a vertical scroll bar.
///
/// Line-wrapping is enabled; default alignment is derived from the script
/// (usually top-left).
pub type ScrollLabel<T> = ScrollText<(), T>;
