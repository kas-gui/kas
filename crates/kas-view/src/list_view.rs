// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view controller

use crate::{DataClerk, DataKey, Driver, SelectionMode, SelectionMsg};
use kas::NavAdvance;
use kas::event::components::ScrollComponent;
use kas::event::{Command, CursorIcon, FocusSource, Scroll, TimerHandle};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::theme::SelectionStyle;
#[allow(unused)] // doc links
use kas_widgets::ScrollBars;
use linear_map::set::LinearSet;
use std::fmt::Debug;
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
                    cx.depress_with_key(self.id(), code);
                    cx.push(kas::messages::Select);
                    Used
                }
                _ => Unused,
            }
        }
    }
}

#[autoimpl(Debug ignore self.item where K: trait)]
struct WidgetData<K, I, V: Driver<K, I>> {
    key: Option<K>,
    item: ListItem<K, I, V>,
}

#[impl_self]
mod ListView {
    /// View controller for 1D indexable data (list)
    ///
    /// This widget generates a view over a list of data items via a
    /// [`DataClerk`]. "View widgets" are constructed via a [`Driver`]
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
    #[derive(Debug)]
    #[widget]
    pub struct ListView<C: DataClerk<usize>, V, D = Direction>
    where
        V: Driver<C::Key, C::Item>,
        D: Directional,
    {
        core: widget_core!(),
        frame_offset: Offset,
        frame_size: Size,
        clerk: C,
        driver: V,
        widgets: Vec<WidgetData<C::Key, C::Item, V>>,
        alloc_len: u32,
        data_len: u32,
        /// The number of widgets in use (cur_len ≤ alloc_len ≤ widgets.len())
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
        sel_mode: SelectionMode,
        sel_style: SelectionStyle,
        // TODO(opt): replace selection list with RangeOrSet type?
        selection: LinearSet<C::Key>,
        press_target: Option<(usize, C::Key)>,
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
    impl<C: DataClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, kas::dir::Left> {
        /// Construct a new instance
        pub fn left(clerk: C, driver: V) -> Self {
            Self::new(clerk, driver)
        }
    }
    impl<C: DataClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, kas::dir::Right> {
        /// Construct a new instance
        pub fn right(clerk: C, driver: V) -> Self {
            Self::new(clerk, driver)
        }
    }
    impl<C: DataClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, kas::dir::Up> {
        /// Construct a new instance
        pub fn up(clerk: C, driver: V) -> Self {
            Self::new(clerk, driver)
        }
    }
    impl<C: DataClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, kas::dir::Down> {
        /// Construct a new instance
        pub fn down(clerk: C, driver: V) -> Self {
            Self::new(clerk, driver)
        }
    }
    impl<C: DataClerk<usize>, V: Driver<C::Key, C::Item>> ListView<C, V, Direction> {
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
                alloc_len: 0,
                data_len: 0,
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
            self.selection.retain(|key| {
                self.widgets
                    .iter()
                    .any(|widget| widget.key.as_ref() == Some(key))
            });
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
                if let Some(ref key) = w.key {
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

        fn position_solver(&self) -> PositionSolver {
            let data_len: usize = self.data_len.cast();
            let cur_len: usize = self.cur_len.cast();
            let mut first_data: usize = self.first_data.cast();
            let mut skip = Offset::ZERO;
            skip.set_component(self.direction, self.skip);

            let mut pos_start = self.rect().pos + self.frame_offset;
            if self.direction.is_reversed() {
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

        // If full, call cx.update on all view widgets
        fn update_widgets(&mut self, cx: &mut ConfigCx, data: &C::Data, full: bool) {
            let time = Instant::now();

            let offset = u64::conv(self.scroll_offset().extract(self.direction));
            let mut first_data = usize::conv(offset / u64::conv(self.skip));

            let data_len: usize = self.data_len.cast();
            let cur_len: usize = data_len.min(self.alloc_len.cast());
            first_data = first_data.min(data_len - cur_len);
            self.cur_len = cur_len.cast();
            debug_assert!(usize::conv(self.cur_len) <= self.widgets.len());
            self.first_data = first_data.cast();

            let range = first_data..(first_data + cur_len);
            self.clerk.prepare_range(cx, self.id(), data, range);

            let solver = self.position_solver();
            for i in solver.data_range() {
                let Some(key) = self.clerk.key(data, i) else {
                    continue;
                };
                let id = key.make_id(self.id_ref());
                let w = &mut self.widgets[i % solver.cur_len];
                if w.key.as_ref() != Some(&key) {
                    w.item.index = i;
                    self.driver.set_key(&mut w.item.inner, &key);

                    if let Some(item) = self.clerk.item(data, &key) {
                        cx.configure(w.item.as_node(item), id);

                        solve_size_rules(
                            &mut w.item,
                            cx.size_cx(),
                            Some(self.child_size.0),
                            Some(self.child_size.1),
                        );
                        w.key = Some(key);
                    } else {
                        w.key = None; // disables drawing and clicking
                    }
                } else if full && let Some(item) = self.clerk.item(data, &key) {
                    cx.update(w.item.as_node(item));
                }

                if w.key.is_some() {
                    w.item.set_rect(cx, solver.rect(i), self.align_hints);
                }
            }

            let dur = (Instant::now() - time).as_micros();
            log::trace!(target: "kas_perf::view::list_view", "update_widgets: {dur}μs");
        }

        fn update_content_size(&mut self, cx: &mut ConfigCx) {
            let data_len: i32 = self.data_len.cast();
            let view_size = self.rect().size - self.frame_size;
            let mut content_size = view_size;
            content_size.set_component(
                self.direction,
                (self.skip * data_len - self.child_inter_margin).max(0),
            );
            let action = self.scroll.set_sizes(view_size, content_size);
            cx.action(self, action);
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            // TODO: support scrolling on the other axis by clamping the min size like ScrollRegion?

            let data_len: i32 = self.data_len.cast();
            let inner_size = (size - self.frame_size).extract(self.direction());
            let child_size =
                (inner_size / self.ideal_visible).clamp(self.child_size_min, self.child_size_ideal);
            let m = self.child_inter_margin;
            let step = child_size + m;
            let content_size = (step * data_len - m).max(0);
            if self.direction.is_horizontal() {
                (content_size > inner_size, false)
            } else {
                (false, content_size > inner_size)
            }
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

            self.child_size_min = i32::MAX;
            let mut rules = SizeRules::EMPTY;
            for w in self.widgets.iter_mut() {
                if w.key.is_some() {
                    let child_rules = w.item.size_rules(sizer.re(), axis);
                    if axis.is_vertical() == self.direction.is_vertical() {
                        self.child_size_min = self.child_size_min.min(child_rules.min_size());
                    }
                    rules = rules.max(child_rules);
                }
            }
            if self.child_size_min == i32::MAX {
                self.child_size_min = 1;
            }
            self.child_size_min = self.child_size_min.max(1);

            if axis.is_vertical() == self.direction.is_vertical() {
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

            if skip == 0 {
                self.skip = 1; // avoid divide by 0
                self.alloc_len = 0;
                return;
            }
            let req_widgets = usize::conv((size + skip - 1) / skip + 1);
            self.alloc_len = req_widgets.cast();

            let avail_widgets = self.widgets.len();
            if avail_widgets < req_widgets {
                log::debug!(
                    "set_rect: allocating widgets (old len = {avail_widgets}, new = {req_widgets})",
                );
                self.widgets.reserve(req_widgets - avail_widgets);
                let key = C::Key::default();
                for _ in avail_widgets..req_widgets {
                    let item = ListItem::new(self.driver.make(&key));
                    self.widgets.push(WidgetData { key: None, item });
                }

                cx.request_frame_timer(self.id(), TIMER_UPDATE_WIDGETS);
            }

            // Call set_rect on children. (This might sometimes be unnecessary,
            // except that the Layout::set_rect specification requires this
            // action and we cannot guarantee that the requested
            // TIMER_UPDATE_WIDGETS event will be immediately.)
            let solver = self.position_solver();
            for i in 0..solver.cur_len {
                let i = solver.first_data + i;
                let w = &mut self.widgets[i % solver.cur_len];
                if w.key.is_some() {
                    w.item.set_rect(cx, solver.rect(i), self.align_hints);
                }
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            let offset = self.scroll_offset();
            draw.with_clip_region(self.rect(), offset, |mut draw| {
                for child in &self.widgets[..self.cur_len.cast()] {
                    if let Some(ref key) = child.key {
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
                len: Some(self.data_len.cast()),
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
                .filter(|w| w.key.is_some())
                .map(|w| w.item.as_tile())
        }
        fn find_child_index(&self, id: &Id) -> Option<usize> {
            let key = C::Key::reconstruct_key(self.id_ref(), id);
            if key.is_some() {
                let num = self.cur_len.cast();
                for (i, w) in self.widgets[..num].iter().enumerate() {
                    if key == w.key {
                        return Some(i);
                    }
                }
            }
            None
        }

        #[inline]
        fn translation(&self, _: usize) -> Offset {
            self.scroll_offset()
        }

        fn probe(&self, coord: Coord) -> Id {
            if self.scroll.is_kinetic_scrolling() {
                return self.id();
            }

            let coord = coord + self.scroll.offset();
            for child in &self.widgets[..self.cur_len.cast()] {
                if child.key.is_some()
                    && let Some(id) = child.item.try_probe(coord)
                {
                    return id;
                }
            }
            self.id()
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
            // We configure children in update_widgets and do not want this method to be called
            unimplemented!()
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            if self.widgets.is_empty() {
                // Initial configure: ensure some widgets are loaded to allow
                // better sizing of self.
                self.skip = 1; // hack: avoid div by 0

                let len = self.ideal_visible.cast();
                let key = C::Key::default();
                self.widgets.resize_with(len, || WidgetData {
                    key: None,
                    item: ListItem::new(self.driver.make(&key)),
                });
                self.alloc_len = len.cast();
            }

            cx.register_nav_fallback(self.id());
        }

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            let id = self.id();
            for w in &mut self.widgets {
                if let Some(ref key) = w.key
                    && let Some(item) = self.clerk.item(data, key)
                {
                    let id = key.make_id(&id);
                    cx.configure(w.item.as_node(item), id);
                }
            }
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &C::Data) {
            self.clerk.update(cx, self.id(), data);
            let data_len = self.clerk.len(data).cast();
            if data_len != self.data_len {
                self.data_len = data_len;
                // We must call at least SET_RECT to update scrollable region
                // RESIZE allows recalculation of child widget size which may
                // have been zero if no data was initially available!
                // TODO(opt): we may not need to resize here.
                cx.resize(&self);
            }

            self.update_widgets(cx, data, true);

            self.update_content_size(cx);
        }

        fn update_recurse(&mut self, _: &mut ConfigCx, _: &Self::Data) {}

        fn handle_event(&mut self, cx: &mut EventCx, data: &C::Data, event: Event) -> IsUsed {
            let mut is_used = match event {
                Event::Command(cmd, _) => {
                    let last = self.clerk.len(data).wrapping_sub(1);
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
                        let act = self.scroll.focus_rect(cx, solver.rect(i_data), self.rect());
                        if !act.is_empty() {
                            cx.action(&self, act);
                            self.update_widgets(&mut cx.config_cx(), data, false);
                        }
                        let index = i_data % usize::conv(self.cur_len);
                        let w = &self.widgets[index];
                        if w.key.is_some() {
                            cx.next_nav_focus(w.item.id(), false, FocusSource::Key);
                        }
                        Used
                    } else {
                        Unused
                    };
                }
                Event::PressStart { ref press }
                    if press.is_primary() && cx.config().event().mouse_nav_focus() =>
                {
                    if let Some(index) = cx.last_child() {
                        self.press_target = self.widgets[index].key.clone().map(|k| (index, k));
                    }
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if w.key.as_ref().map(|k| k == key).unwrap_or(false) {
                            cx.next_nav_focus(w.item.id(), false, FocusSource::Pointer);
                        }
                    }

                    // Press may also be grabbed by scroll component (replacing
                    // this). Either way we can select on PressEnd.
                    press
                        .grab(self.id(), kas::event::GrabMode::Click)
                        .complete(cx)
                }
                Event::PressEnd { ref press, success } if press.is_primary() => {
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if success
                            && !matches!(self.sel_mode, SelectionMode::None)
                            && !self.scroll.is_kinetic_scrolling()
                            && w.key.as_ref().map(|k| k == key).unwrap_or(false)
                            && w.item.rect().contains(press.coord + self.scroll.offset())
                        {
                            cx.push(kas::messages::Select);
                        }
                    }
                    Used
                }
                Event::Timer(TIMER_UPDATE_WIDGETS) => {
                    self.update_widgets(&mut cx.config_cx(), data, false);
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
                opt_key = match self.widgets[index].key.as_ref() {
                    Some(k) => Some(k.clone()),
                    None => return, // should be unreachable
                };
            }

            self.clerk
                .handle_messages(cx, self.id(), data, opt_key.as_ref());

            if let Some(kas::messages::Select) = cx.try_pop() {
                let key = match opt_key {
                    Some(key) => key,
                    None => match self.press_target.as_ref() {
                        Some((_, k)) => k.clone(),
                        None => return,
                    },
                };

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
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, data: &C::Data, scroll: Scroll) {
            self.scroll.scroll(cx, self.id(), self.rect(), scroll);
            self.update_widgets(&mut cx.config_cx(), data, false);
        }
    }

    // Direct implementation of this trait outside of Kas code is not supported!
    impl Widget for Self {
        type Data = C::Data;

        fn child_node<'n>(&'n mut self, data: &'n C::Data, index: usize) -> Option<Node<'n>> {
            if let Some(w) = self.widgets.get_mut(index)
                && let Some(ref key) = w.key
                && let Some(item) = self.clerk.item(data, key)
            {
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
                let last_data = self.clerk.len(data) - 1;
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

                let act = self
                    .scroll
                    .self_focus_rect(solver.rect(data_index), self.rect());
                if !act.is_empty() {
                    cx.action(&self, act);
                    self.update_widgets(cx, data, false);
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
    /// Data range
    fn data_range(&self) -> std::ops::Range<usize> {
        self.first_data..(self.first_data + self.cur_len)
    }

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
