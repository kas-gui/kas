// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view controller

use crate::clerk::{Changes, Key, TokenClerk};
use crate::{Driver, SelectionMode, SelectionMsg, Update};
use kas::event::components::ScrollComponent;
use kas::event::{CursorIcon, FocusSource, NavAdvance, Scroll, TimerHandle};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::theme::SelectionStyle;
#[allow(unused)] // doc links
use kas_widgets::ScrollBars;
use linear_map::set::LinearSet;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::ops::Range;
use std::time::Instant;

const TIMER_UPDATE_WIDGETS: TimerHandle = TimerHandle::new(1, true);

#[impl_self]
mod ListItem {
    /// A wrapper for selectable items
    ///
    /// This widget adds a thin frame around contents, supporting navigation
    /// focus and activation.
    ///
    /// # Messages
    ///
    /// When activated, this widget pushes [`Select`] to the message stack.
    ///
    /// [`Select`]: kas::messages::Select
    #[widget]
    #[layout(frame!(self.inner).with_style(kas::theme::FrameStyle::NavFocus))]
    struct ListItem<K, I, V: Driver<K, I>> {
        core: widget_core!(),
        index: usize,
        selected: Option<bool>,
        /// The inner widget
        #[widget]
        inner: V::Widget,
    }

    impl Self {
        /// Construct a frame
        #[inline]
        fn new(inner: V::Widget) -> Self {
            ListItem {
                core: Default::default(),
                index: 0,
                selected: None,
                inner,
            }
        }
    }

    impl Tile for Self {
        fn role(&self, cx: &mut dyn RoleCx) -> Role<'_> {
            if let Some(label) = V::label(&self.inner) {
                cx.set_label(label);
            }
            Role::OptionListItem {
                index: Some(self.index),
                selected: self.selected,
            }
        }

        fn navigable(&self) -> bool {
            V::navigable(&self.inner)
        }
    }

    impl Events for Self {
        type Data = I;

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, code) if cmd.is_activate() => {
                    cx.depress_with_key(&self, code);
                    cx.push(kas::messages::Select);
                    Used
                }
                _ => Unused,
            }
        }
    }
}

#[autoimpl(Debug ignore self.item where C::Token: trait)]
struct WidgetData<C: TokenClerk<usize>, V: Driver<C::Key, C::Item>> {
    token: Option<C::Token>,
    is_mock: bool,
    item: ListItem<C::Key, C::Item, V>,
}

impl<C: TokenClerk<usize>, V: Driver<C::Key, C::Item>> WidgetData<C, V> {
    fn key(&self) -> Option<&C::Key> {
        self.token.as_ref().map(Borrow::borrow)
    }
}

#[impl_self]
mod ListView {
    /// View controller for 1D indexable data (list)
    ///
    /// This widget generates a view over a list of data items via a
    /// [`TokenClerk`]. "View widgets" are constructed via a [`Driver`]
    /// to represent visible data items. These view widgets are reassigned as
    /// required when the list is scrolled, keeping the number of widgets in
    /// use roughly proportional to the number of data items within the view.
    ///
    /// Each view widget has an [`Id`] corresponding to its current data
    /// item, and may handle events and emit messages like other widegts.
    /// See [`Driver`] documentation for more on event handling.
    ///
    /// This widget is [`Scrollable`], supporting keyboard, wheel and drag
    /// scrolling. You may wish to wrap this widget with [`ScrollBars`].
    ///
    /// Optionally, data items may be selected; see [`Self::set_selection_mode`].
    /// If enabled, [`SelectionMsg`] messages are reported; view widgets may
    /// emit [`kas::messages::Select`] to have themselves be selected.
    ///
    /// ### Messages
    ///
    /// [`kas::messages::SetScrollOffset`] may be used to set the scroll offset.
    #[widget]
    pub struct ListView<C: TokenClerk<usize>, V, D = Direction>
    where
        V: Driver<C::Key, C::Item>,
        D: Directional,
    {
        core: widget_core!(),
        frame_offset: Offset,
        frame_size: Size,
        clerk: C,
        driver: V,
        widgets: Vec<WidgetData<C, V>>,
        data_len: u32,
        token_update: Update,
        rect_update: bool,
        len_is_known: bool,
        /// The number of widgets in use (cur_len ≤ widgets.len())
        cur_len: u32,
        /// First data item mapped to a widget
        first_data: u32,
        direction: D,
        align_hints: AlignHints,
        ideal_visible: i32,
        child_size_min: i32,
        child_size_ideal: i32,
        child_inter_margin: i32,
        skip: i32,
        child_size: Size,
        scroll: ScrollComponent,
        // Widget translation is scroll.offset() + virtual_offset
        virtual_offset: i32,
        sel_mode: SelectionMode,
        sel_style: SelectionStyle,
        // TODO(opt): replace selection list with RangeOrSet type?
        selection: LinearSet<C::Key>,
        press_target: Option<(usize, C::Key)>,
    }

    impl Default for Self
    where
        C: Default,
        V: Default,
        D: Default,
    {
        fn default() -> Self {
            Self::new(C::default(), V::default())
        }
    }
    impl Self
    where
        D: Default,
    {
        /// Construct a new instance
        pub fn new(clerk: C, driver: V) -> Self {
            Self::new_dir(clerk, driver, D::default())
        }
    }
    impl<C: TokenClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, kas::dir::Left> {
        /// Construct a new instance
        pub fn left(clerk: C, driver: V) -> Self {
            Self::new(clerk, driver)
        }
    }
    impl<C: TokenClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, kas::dir::Right> {
        /// Construct a new instance
        pub fn right(clerk: C, driver: V) -> Self {
            Self::new(clerk, driver)
        }
    }
    impl<C: TokenClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, kas::dir::Up> {
        /// Construct a new instance
        pub fn up(clerk: C, driver: V) -> Self {
            Self::new(clerk, driver)
        }
    }
    impl<C: TokenClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, kas::dir::Down> {
        /// Construct a new instance
        pub fn down(clerk: C, driver: V) -> Self {
            Self::new(clerk, driver)
        }
    }
    impl<C: TokenClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, Direction> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, cx: &mut EventState, direction: Direction) {
            if direction != self.direction {
                self.direction = direction;
                cx.action(self, Action::SET_RECT);
            }
        }
    }

    impl Self {
        /// Construct a new instance
        pub fn new_dir(clerk: C, driver: V, direction: D) -> Self {
            ListView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                clerk,
                driver,
                widgets: Default::default(),
                data_len: 0,
                token_update: Update::None,
                rect_update: false,
                len_is_known: false,
                cur_len: 0,
                first_data: 0,
                direction,
                align_hints: Default::default(),
                ideal_visible: 5,
                child_size_min: 0,
                child_size_ideal: 0,
                child_inter_margin: 0,
                skip: 1,
                child_size: Size::ZERO,
                scroll: Default::default(),
                virtual_offset: 0,
                sel_mode: SelectionMode::None,
                sel_style: SelectionStyle::Highlight,
                selection: Default::default(),
                press_target: None,
            }
        }

        /// Access the data clerk
        pub fn clerk(&self) -> &C {
            &self.clerk
        }

        /// Access the data clerk (mutably)
        ///
        /// Changes to the clerk must be notified with an update to the
        /// `GridView`, for example using [`ConfigCx::update`],
        /// [`EventCx::update`] or [`Action::UPDATE`].
        pub fn clerk_mut(&mut self) -> &mut C {
            &mut self.clerk
        }

        /// Get the range of visible data items
        ///
        /// Data items within this range may be visible (or should at least be
        /// allocated some pixel within the controller's view).
        pub fn view_range(&self) -> Range<usize> {
            let start: usize = self.first_data.cast();
            let end = start + usize::conv(self.cur_len);
            start..end
        }

        /// Get the current selection mode
        pub fn selection_mode(&self) -> SelectionMode {
            self.sel_mode
        }
        /// Set the current selection mode
        ///
        /// By default, selection is disabled. If enabled, items may be selected
        /// and deselected via mouse-click/touch or via a view widget emitting
        /// [`Select`].
        ///
        /// On selection and deselection, a [`SelectionMsg`] message is emitted.
        ///
        /// [`Select`]: kas::messages::Select
        pub fn set_selection_mode(&mut self, cx: &mut EventState, mode: SelectionMode) {
            self.sel_mode = mode;
            match mode {
                SelectionMode::None if !self.selection.is_empty() => {
                    self.selection.clear();
                    self.update_selected_items();
                    cx.redraw(self);
                }
                SelectionMode::Single if self.selection.len() > 1 => {
                    if let Some(first) = self.selection.iter().next().cloned() {
                        self.selection.retain(|item| *item == first);
                    }
                    self.update_selected_items();
                    cx.redraw(self);
                }
                _ => (),
            }
        }
        /// Set the initial selection mode (inline)
        ///
        /// See [`Self::set_selection_mode`] documentation.
        #[must_use]
        pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
            debug_assert!(self.selection.is_empty());
            self.sel_mode = mode;
            self
        }

        /// Get the current selection style
        pub fn selection_style(&self) -> SelectionStyle {
            self.sel_style
        }
        /// Set the current selection style
        ///
        /// By default, [`SelectionStyle::Highlight`] is used. Other modes may
        /// add margin between elements.
        pub fn set_selection_style(&mut self, cx: &mut EventState, style: SelectionStyle) {
            if style.is_external() != self.sel_style.is_external() {
                cx.resize(&self);
            };
            self.sel_style = style;
        }
        /// Set the selection style (inline)
        ///
        /// See [`Self::set_selection_style`] documentation.
        #[must_use]
        pub fn with_selection_style(mut self, style: SelectionStyle) -> Self {
            self.sel_style = style;
            self
        }

        /// Read the list of selected entries
        ///
        /// With mode [`SelectionMode::Single`] this may contain zero or one entry;
        /// use `selected_iter().next()` to extract only the first (optional) entry.
        pub fn selected_iter(&'_ self) -> impl Iterator<Item = &'_ C::Key> + '_ {
            self.selection.iter()
        }

        /// Check whether an entry is selected
        pub fn is_selected(&self, key: &C::Key) -> bool {
            self.selection.contains(key)
        }

        /// Clear all selected items
        pub fn clear_selected(&mut self, cx: &mut EventState) {
            if !self.selection.is_empty() {
                self.selection.clear();
                self.update_selected_items();
                cx.redraw(self);
            }
        }

        /// Directly select an item
        ///
        /// Does nothing if [`Self::selection_mode`] is [`SelectionMode::None`].
        /// Does not verify the validity of `key`.
        /// Does not send [`SelectionMsg`] messages.
        ///
        /// Returns `true` if newly selected, `false` if
        /// already selected. Fails if selection mode does not permit selection
        /// or if the key is invalid.
        pub fn select(&mut self, cx: &mut EventState, key: C::Key) -> bool {
            match self.sel_mode {
                SelectionMode::None => return false,
                SelectionMode::Single => self.selection.clear(),
                _ => (),
            }
            let r = self.selection.insert(key);
            if r {
                self.update_selected_items();
                cx.redraw(self);
            }
            r
        }

        /// Directly deselect an item
        ///
        /// Returns `true` if deselected, `false` if not
        /// previously selected or if the key is invalid.
        pub fn deselect(&mut self, cx: &mut EventState, key: &C::Key) -> bool {
            let r = self.selection.remove(key);
            if r {
                self.update_selected_items();
                cx.redraw(self);
            }
            r
        }

        /// Deselect all unavailable items
        ///
        /// By default, the set of selected keys persists when items filtered or
        /// removed the data set. This allows the selection to survive changing
        /// filters. To avoid this behaviour, call this method on update.
        pub fn deselect_unavailable(&mut self, cx: &mut EventState) {
            let len = self.selection.len();
            self.selection
                .retain(|key| self.widgets.iter().any(|widget| widget.key() == Some(key)));
            self.update_selected_items();
            if len != self.selection.len() {
                cx.redraw(self);
            }
        }

        // TODO(opt): some usages only require one item be updated
        fn update_selected_items(&mut self) {
            let unselected = match self.sel_mode {
                SelectionMode::None | SelectionMode::Single => None,
                SelectionMode::Multiple => Some(false),
            };
            for w in &mut self.widgets {
                if let Some(key) = w.key() {
                    if self.selection.contains(key) {
                        w.item.selected = Some(true);
                    } else {
                        w.item.selected = unselected;
                    }
                }
            }
        }

        /// Get the direction of contents
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Set the preferred number of items visible (inline)
        ///
        /// This affects the (ideal) size request and whether children are sized
        /// according to their ideal or minimum size but not the minimum size.
        #[must_use]
        pub fn with_num_visible(mut self, number: i32) -> Self {
            self.ideal_visible = number;
            self
        }

        #[inline]
        fn virtual_offset(&self) -> Offset {
            match self.direction.is_vertical() {
                false => Offset(self.virtual_offset, 0),
                true => Offset(0, self.virtual_offset),
            }
        }

        fn position_solver(&self) -> PositionSolver {
            let cur_len: usize = self.cur_len.cast();
            let mut first_data: usize = self.first_data.cast();
            let mut skip = Offset::ZERO;
            skip.set_component(self.direction, self.skip);

            let mut pos_start = self.rect().pos + self.frame_offset + self.virtual_offset();
            if self.direction.is_reversed() && self.len_is_known {
                let data_len: usize = self.data_len.cast();
                first_data = (data_len - first_data).saturating_sub(cur_len);
                pos_start += skip * i32::conv(data_len.saturating_sub(1));
                skip = skip * -1;
            }

            PositionSolver {
                pos_start,
                skip,
                size: self.child_size,
                first_data,
                cur_len,
            }
        }

        // Call after scrolling to re-map widgets (if required)
        fn post_scroll(&mut self, cx: &mut ConfigCx, data: &C::Data) {
            let offset = self.scroll_offset().extract(self.direction);
            let first_data = usize::conv(u64::conv(offset) / u64::conv(self.skip));

            let alloc_len = self.widgets.len();
            let data_len;
            if !self.len_is_known {
                let lbound = first_data + 2 * alloc_len;
                let result = self.clerk.len(data, lbound);
                self.len_is_known = result.is_known();
                data_len = result.len();
                if data_len != usize::conv(self.data_len) {
                    self.data_len = data_len.cast();
                    self.update_content_size(cx);
                }
            } else {
                data_len = self.data_len.cast();
            }
            let cur_len = data_len.min(alloc_len);
            let first_data = first_data.min(data_len - cur_len);

            let old_start = self.first_data.cast();
            let old_end = old_start + usize::conv(self.cur_len);
            let (mut start, mut end) = (first_data, first_data + cur_len);

            let virtual_offset = -(offset & 0x7FF0_0000);
            if virtual_offset != self.virtual_offset {
                self.virtual_offset = virtual_offset;
                self.rect_update = true;
            } else if self.rect_update || self.token_update != Update::None {
                // This forces an update to all widgets
            } else if start >= old_start {
                start = start.max(old_end);
            } else if end <= old_end {
                end = end.min(old_start);
            }

            debug_assert!(cur_len <= self.widgets.len());
            self.cur_len = cur_len.cast();
            self.first_data = first_data.cast();

            if start < end {
                self.map_view_widgets(cx, data, start..end);
            }
        }

        // Assign view widgets to data as required and set their rects
        //
        // View widgets are configured and sized if assigned a new data item.
        fn map_view_widgets(&mut self, cx: &mut ConfigCx, data: &C::Data, range: Range<usize>) {
            let time = Instant::now();

            self.clerk
                .prepare_range(cx, self.id(), self.view_range(), data, range.clone());

            let id = self.id();

            let solver = self.position_solver();
            for i in range.clone() {
                let w = &mut self.widgets[i % solver.cur_len];

                let force = self.token_update != Update::None;
                let changes = self.clerk.update_token(data, i, force, &mut w.token);
                w.is_mock = false;
                let Some(token) = w.token.as_ref() else {
                    continue;
                };

                let mut rect_update = self.rect_update;
                if changes.key() || self.token_update == Update::Configure {
                    w.item.index = i;
                    // TODO(opt): some impls of Driver::set_key do nothing
                    // and do not need re-configure (beyond the first).
                    self.driver.set_key(&mut w.item.inner, token.borrow());

                    let item = self.clerk.item(data, token);
                    let id = token.borrow().make_id(&id);
                    cx.configure(w.item.as_node(item), id);

                    solve_size_rules(
                        &mut w.item,
                        cx.size_cx(),
                        Some(self.child_size.0),
                        Some(self.child_size.1),
                    );
                    rect_update = true;
                } else if changes.item() {
                    let item = self.clerk.item(data, token);
                    cx.update(w.item.as_node(item));
                }

                if rect_update {
                    w.item.set_rect(cx, solver.rect(i), self.align_hints);
                }
            }

            self.token_update = Update::None;
            self.rect_update = false;

            let dur = (Instant::now() - time).as_micros();
            log::debug!(
                target: "kas_perf::view::list_view",
                "map_view_widgets: {} view widgets in: {dur}μs",
                range.len(),
            );
        }

        // Handle a data clerk update
        fn handle_clerk_update(
            &mut self,
            cx: &mut ConfigCx,
            data: &C::Data,
            changes: Changes<usize>,
        ) {
            let start: usize = self.first_data.cast();
            let end = start + usize::conv(self.cur_len);

            let lbound = usize::conv(start) + 2 * self.widgets.len();
            let data_len = self.clerk.len(data, lbound);
            self.len_is_known = data_len.is_known();
            let data_len = data_len.len().cast();

            if data_len != self.data_len {
                let old_len = self.data_len;
                self.data_len = data_len;

                let cur_len = data_len.min(self.widgets.len().cast());
                if self.cur_len != cur_len || end >= usize::conv(data_len.min(old_len)) {
                    self.cur_len = cur_len;
                    self.token_update = self.token_update.max(Update::Token);
                    return self.post_scroll(cx, data);
                }
            }

            let range = match changes {
                Changes::None | Changes::NoPreparedItems => 0..0,
                Changes::Range(range) => start.max(range.start)..end.min(range.end),
                Changes::Any => start..end,
            };

            if !range.is_empty() {
                self.token_update = self.token_update.max(Update::Token);
                self.map_view_widgets(cx, data, range);
            }
        }

        /// Returns true if anything changed
        fn update_content_size(&mut self, cx: &mut ConfigCx) -> bool {
            let data_len: i32 = self.data_len.cast();
            let view_size = self.rect().size - self.frame_size;
            let mut content_size = view_size;
            content_size.set_component(
                self.direction,
                (self.skip * data_len - self.child_inter_margin).max(0),
            );
            let action = self.scroll.set_sizes(view_size, content_size);
            cx.action(self, action);
            !action.is_empty()
        }
    }

    impl Scrollable for Self {
        fn content_size(&self) -> Size {
            let data_len: i32 = self.data_len.cast();
            let m = self.child_inter_margin;
            let step = self.child_size_ideal + m;
            let mut content_size = Size::ZERO;
            content_size.set_component(self.direction, (step * data_len - m).max(0));
            content_size
        }

        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.scroll.max_offset()
        }

        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.scroll.offset()
        }

        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let action = self.scroll.set_offset(offset);
            if !action.is_empty() {
                cx.action(&self, action);
                cx.request_frame_timer(self.id(), TIMER_UPDATE_WIDGETS);
            }
            self.scroll.offset()
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            // We use an invisible frame for highlighting selections, drawing into the margin
            let inner_margin = if self.sel_style.is_external() {
                sizer.inner_margins().extract(axis)
            } else {
                (0, 0)
            };
            let frame = kas::layout::FrameRules::new(0, inner_margin, (0, 0));

            let other = axis.other().map(|mut size| {
                // Use same logic as in set_rect to find per-child size:
                let other_axis = axis.flipped();
                size -= self.frame_size.extract(other_axis);
                if self.direction.is_horizontal() == other_axis.is_horizontal() {
                    size = (size / self.ideal_visible)
                        .clamp(self.child_size_min, self.child_size_ideal);
                }
                size
            });
            axis = AxisInfo::new(axis.is_vertical(), other);

            let mut rules = SizeRules::EMPTY;
            for w in self.widgets.iter_mut() {
                if w.token.is_some() || w.is_mock {
                    let child_rules = w.item.size_rules(sizer.re(), axis);
                    rules = rules.max(child_rules);
                }
            }
            if axis.is_vertical() == self.direction.is_vertical() {
                self.child_size_min = rules.min_size().max(1);
                self.child_size_ideal = rules.ideal_size().max(sizer.min_element_size());
                let m = rules.margins();
                self.child_inter_margin =
                    m.0.max(m.1).max(inner_margin.0).max(inner_margin.1).cast();
                rules.multiply_with_margin(2, self.ideal_visible);
                rules.set_stretch(rules.stretch().max(Stretch::High));
            } else {
                rules.set_stretch(rules.stretch().max(Stretch::Low));
            }
            let (rules, offset, size) = frame.surround(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            self.align_hints = hints;

            let mut child_size = rect.size - self.frame_size;
            let (size, skip);
            if self.direction.is_horizontal() {
                child_size.0 = (child_size.0 / self.ideal_visible)
                    .clamp(self.child_size_min, self.child_size_ideal);
                size = rect.size.0;
                skip = child_size.0 + self.child_inter_margin;
            } else {
                child_size.1 = (child_size.1 / self.ideal_visible)
                    .clamp(self.child_size_min, self.child_size_ideal);
                size = rect.size.1;
                skip = child_size.1 + self.child_inter_margin;
            }

            self.child_size = child_size;
            self.skip = skip;
            self.update_content_size(cx);

            let req_widgets = if skip == 0 {
                self.skip = 1; // avoid divide by 0
                0
            } else {
                usize::conv((size + skip - 1) / skip + 1)
            };

            let avail_widgets = self.widgets.len();
            if avail_widgets < req_widgets {
                log::debug!(
                    "set_rect: allocating widgets (old len = {avail_widgets}, new = {req_widgets})",
                );
                self.widgets.reserve(req_widgets - avail_widgets);
                let key = C::Key::default();
                for _ in avail_widgets..req_widgets {
                    let item = ListItem::new(self.driver.make(&key));
                    self.widgets.push(WidgetData {
                        token: None,
                        is_mock: false,
                        item,
                    });
                }
            }

            // Call set_rect on children. (This might sometimes be unnecessary,
            // except that the Layout::set_rect specification requires this
            // action and we cannot guarantee that the requested
            // TIMER_UPDATE_WIDGETS event will be immediately.)
            let solver = self.position_solver();
            for i in 0..solver.cur_len {
                let i = solver.first_data + i;
                let w = &mut self.widgets[i % solver.cur_len];
                if w.token.is_some() {
                    w.item.set_rect(cx, solver.rect(i), self.align_hints);
                }
            }

            self.rect_update = true;
            cx.request_frame_timer(self.id(), TIMER_UPDATE_WIDGETS);
        }

        fn draw(&self, mut draw: DrawCx) {
            // We use a new pass to clip and offset scrolled content:
            let offset = self.scroll_offset() + self.virtual_offset();
            draw.with_clip_region(self.rect(), offset, |mut draw| {
                for child in &self.widgets[..self.cur_len.cast()] {
                    if let Some(key) = child.key() {
                        if self.selection.contains(key) {
                            draw.selection(child.item.rect(), self.sel_style);
                        }
                        child.item.draw(draw.re());
                    }
                }
            });
        }
    }

    impl Tile for Self {
        fn role(&self, cx: &mut dyn RoleCx) -> Role<'_> {
            cx.set_scroll_offset(self.scroll_offset(), self.max_scroll_offset());
            Role::OptionList {
                len: self.len_is_known.then(|| self.data_len.cast()),
                direction: self.direction.as_direction(),
            }
        }

        #[inline]
        fn child_indices(&self) -> ChildIndices {
            (0..self.cur_len.cast()).into()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Tile> {
            self.widgets
                .get(index)
                .filter(|w| w.token.is_some())
                .map(|w| w.item.as_tile())
        }
        fn find_child_index(&self, id: &Id) -> Option<usize> {
            let key = C::Key::reconstruct_key(self.id_ref(), id);
            if key.is_some() {
                let num = self.cur_len.cast();
                for (i, w) in self.widgets[..num].iter().enumerate() {
                    if key.as_ref() == w.key() {
                        return Some(i);
                    }
                }
            }
            None
        }

        #[inline]
        fn translation(&self, _: usize) -> Offset {
            self.scroll_offset() + self.virtual_offset()
        }
    }

    impl Events for Self {
        fn mouse_over_icon(&self) -> Option<CursorIcon> {
            self.scroll
                .is_kinetic_scrolling()
                .then_some(CursorIcon::AllScroll)
        }

        #[inline]
        fn make_child_id(&mut self, _: usize) -> Id {
            // We configure children in map_view_widgets and do not want this method to be called
            unimplemented!()
        }

        fn probe(&self, coord: Coord) -> Id {
            if self.scroll.is_kinetic_scrolling() {
                return self.id();
            }

            let coord = coord + self.translation(0);
            for child in &self.widgets[..self.cur_len.cast()] {
                if child.token.is_some()
                    && let Some(id) = child.item.try_probe(coord)
                {
                    return id;
                }
            }
            self.id()
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.register_nav_fallback(self.id());
        }

        fn configure_recurse(&mut self, _: &mut ConfigCx, _: &Self::Data) {
            if self.widgets.is_empty() {
                // Ensure alloc_len > 0 for initial sizing
                self.skip = 1; // hack: avoid div by 0
                let len = self.ideal_visible.cast();
                let key = C::Key::default();
                self.widgets.resize_with(len, || WidgetData {
                    token: None,
                    is_mock: false,
                    item: ListItem::new(self.driver.make(&key)),
                });
            } else {
                // Force reconfiguration:
                for w in &mut self.widgets {
                    w.token = None;
                }
            }
            self.token_update = Update::Configure;
            // Self::update() will be called next
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &C::Data) {
            let changes = self.clerk.update(cx, self.id(), self.view_range(), data);
            if self.token_update != Update::None {
                self.post_scroll(cx, data);
            } else if changes != Changes::None {
                self.handle_clerk_update(cx, data, changes);
            }

            let id = self.id();
            if self.cur_len == 0
                && let Some(w) = self.widgets.get_mut(0)
                && w.token.is_none()
                && !w.is_mock
                && let Some(item) = self.clerk.mock_item(data)
            {
                // Construct a mock widget for initial sizing
                cx.configure(w.item.as_node(&item), id);
                w.is_mock = true;
            }
        }

        fn update_recurse(&mut self, _: &mut ConfigCx, _: &Self::Data) {}

        fn handle_event(&mut self, cx: &mut EventCx, data: &C::Data, event: Event) -> IsUsed {
            let mut is_used = match event {
                Event::Command(cmd, _) => {
                    let last = usize::conv(self.data_len).wrapping_sub(1);
                    if last == usize::MAX {
                        return Unused;
                    }

                    let solver = self.position_solver();
                    let cur = match cx.nav_focus().and_then(|id| self.find_child_index(id)) {
                        Some(index) => solver.child_to_data(index),
                        None => return Unused,
                    };
                    let is_vert = self.direction.is_vertical();
                    let len = solver.cur_len;

                    use Command as C;
                    let data_index = match cmd {
                        C::Home | C::DocHome => Some(0),
                        C::End | C::DocEnd => Some(last),
                        C::Left | C::WordLeft if !is_vert && cur > 0 => Some(cur - 1),
                        C::Up if is_vert && cur > 0 => Some(cur - 1),
                        C::Right | C::WordRight if !is_vert && cur < last => Some(cur + 1),
                        C::Down if is_vert && cur < last => Some(cur + 1),
                        C::PageUp if cur > 0 => Some(cur.saturating_sub(len / 2)),
                        C::PageDown if cur < last => Some((cur + len / 2).min(last)),
                        // TODO: C::ViewUp, ...
                        _ => None,
                    };
                    return if let Some(i_data) = data_index {
                        // Set nav focus to i_data and update scroll position
                        let rect = solver.rect(i_data) - self.virtual_offset();
                        let act = self.scroll.focus_rect(cx, rect, self.rect());
                        if !act.is_empty() {
                            cx.action(&self, act);
                            self.post_scroll(&mut cx.config_cx(), data);
                        }
                        let index = i_data % usize::conv(self.cur_len);
                        let w = &self.widgets[index];
                        if w.token.is_some() {
                            cx.next_nav_focus(w.item.id(), false, FocusSource::Key);
                        }
                        Used
                    } else {
                        Unused
                    };
                }
                Event::PressStart(ref press)
                    if press.is_primary() && cx.config().event().mouse_nav_focus() =>
                {
                    if let Some(index) = cx.last_child() {
                        self.press_target = self.widgets[index].key().map(|k| (index, k.clone()));
                    }
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if w.key() == Some(key) {
                            cx.next_nav_focus(w.item.id(), false, FocusSource::Pointer);
                        }
                    }

                    // Press may also be grabbed by scroll component (replacing
                    // this). Either way we can select on PressEnd.
                    press.grab_click(self.id()).complete(cx)
                }
                Event::PressEnd { ref press, success } if press.is_primary() => {
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if success
                            && !matches!(self.sel_mode, SelectionMode::None)
                            && !self.scroll.is_kinetic_scrolling()
                            && w.key() == Some(key)
                            && w.item.rect().contains(press.coord + self.translation(0))
                        {
                            cx.push(kas::messages::Select);
                        }
                    }
                    Used
                }
                Event::Timer(TIMER_UPDATE_WIDGETS) => {
                    self.post_scroll(&mut cx.config_cx(), data);
                    Used
                }
                _ => Unused, // fall through to scroll handler
            };

            let offset = self.scroll.offset();
            is_used |= self
                .scroll
                .scroll_by_event(cx, event, self.id(), self.rect());
            if offset != self.scroll.offset() {
                // We may process multiple 'moved' events per frame; TIMER_UPDATE_WIDGETS will only
                // be processed once per frame.
                cx.request_frame_timer(self.id(), TIMER_UPDATE_WIDGETS);
            }
            is_used
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &C::Data) {
            if let Some(kas::messages::SetScrollOffset(offset)) = cx.try_pop() {
                self.set_scroll_offset(cx, offset);
                return;
            }

            let mut opt_key = None;
            if let Some(index) = cx.last_child() {
                // Message is from a child
                if let Some(token) = self.widgets.get_mut(index).and_then(|w| w.token.as_mut()) {
                    opt_key = Some(Borrow::<C::Key>::borrow(token).clone());
                } else {
                    return; // should be unreachable
                };
            }

            if let Some(kas::messages::Select) = cx.try_pop() {
                let key = match opt_key {
                    Some(key) => key,
                    None => match self.press_target.as_ref() {
                        Some((_, k)) => k.clone(),
                        None => return,
                    },
                };
                opt_key = None;

                match self.sel_mode {
                    SelectionMode::None => (),
                    SelectionMode::Single => {
                        cx.redraw(&self);
                        self.selection.clear();
                        self.selection.insert(key.clone());
                        self.update_selected_items();
                        cx.push(SelectionMsg::Select(key));
                    }
                    SelectionMode::Multiple => {
                        cx.redraw(&self);
                        if self.selection.remove(&key) {
                            cx.push(SelectionMsg::Deselect(key.clone()));
                        } else {
                            self.selection.insert(key.clone());
                            cx.push(SelectionMsg::Select(key));
                        }
                        self.update_selected_items();
                    }
                }
            }

            let changes =
                self.clerk
                    .handle_messages(cx, self.id(), self.view_range(), data, opt_key);
            if changes != Changes::None {
                self.handle_clerk_update(&mut cx.config_cx(), data, changes);
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, data: &C::Data, scroll: Scroll) {
            self.scroll
                .scroll(cx, self.id(), self.rect(), scroll - self.virtual_offset());
            self.post_scroll(&mut cx.config_cx(), data);
        }
    }

    // Direct implementation of this trait outside of Kas code is not supported!
    impl Widget for Self {
        type Data = C::Data;

        fn child_node<'n>(&'n mut self, data: &'n C::Data, index: usize) -> Option<Node<'n>> {
            if let Some(w) = self.widgets.get_mut(index)
                && let Some(ref token) = w.token
            {
                let item = self.clerk.item(data, token);
                return Some(w.item.as_node(item));
            }

            None
        }

        // Non-standard implementation to allow mapping new children
        fn _nav_next(
            &mut self,
            cx: &mut ConfigCx,
            data: &C::Data,
            focus: Option<&Id>,
            advance: NavAdvance,
        ) -> Option<Id> {
            if cx.is_disabled(self.id_ref()) || self.cur_len == 0 {
                return None;
            }

            let mut child = focus.and_then(|id| self.find_child_index(id));

            if let Some(index) = child {
                let mut opt_id = None;
                let out = &mut opt_id;
                if let Some(mut node) = self.as_node(data).get_child(index) {
                    *out = node._nav_next(cx, focus, advance);
                }
                if let Some(id) = opt_id {
                    return Some(id);
                }
            }

            let reverse = match advance {
                NavAdvance::None => return None,
                NavAdvance::Forward(_) => false,
                NavAdvance::Reverse(_) => true,
            };

            let mut starting_child = child;
            loop {
                let solver = self.position_solver();
                let last_data = usize::conv(self.data_len).wrapping_sub(1);
                let data_index = if let Some(index) = child {
                    let data = solver.child_to_data(index);
                    if !reverse && data < last_data {
                        data + 1
                    } else if reverse && data > 0 {
                        data - 1
                    } else {
                        return None;
                    }
                } else if !reverse {
                    0
                } else {
                    last_data
                };

                let rect = solver.rect(data_index) - self.virtual_offset();
                let act = self.scroll.self_focus_rect(rect, self.rect());
                if !act.is_empty() {
                    cx.action(&self, act);
                    self.post_scroll(cx, data);
                }

                let index = data_index % usize::conv(self.cur_len);

                let mut opt_id = None;
                let out = &mut opt_id;
                if let Some(mut node) = self.as_node(data).get_child(index) {
                    *out = node._nav_next(cx, focus, advance);
                }
                if let Some(id) = opt_id {
                    return Some(id);
                }

                child = Some(index);
                if starting_child == child {
                    return None;
                } else if starting_child.is_none() {
                    starting_child = child;
                }
            }
        }
    }
}

#[derive(Debug)]
struct PositionSolver {
    pos_start: Coord,
    skip: Offset,
    size: Size,
    first_data: usize,
    cur_len: usize,
}

impl PositionSolver {
    /// Map a child index to a data index
    fn child_to_data(&self, index: usize) -> usize {
        let mut data = (self.first_data / self.cur_len) * self.cur_len + index;
        if data < self.first_data {
            data += self.cur_len;
        }
        data
    }

    /// Rect of data item i
    fn rect(&self, i: usize) -> Rect {
        let pos = self.pos_start + self.skip * i32::conv(i);
        Rect::new(pos, self.size)
    }
}
