// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view controller

use crate::{DataKey, Driver, ListData, SelectionMode, SelectionMsg};
use kas::event::components::ScrollComponent;
use kas::event::{Command, FocusSource, Scroll, TimerHandle};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::theme::SelectionStyle;
use kas::NavAdvance;
#[allow(unused)] // doc links
use kas_widgets::ScrollBars;
use linear_map::set::LinearSet;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::time::Instant;

const TIMER_UPDATE_WIDGETS: TimerHandle = TimerHandle::new(1, true);

#[derive(Clone, Debug, Default)]
struct WidgetData<K, W> {
    key: Option<K>,
    widget: W,
}

impl_scope! {
    /// View controller for 1D indexable data (list)
    ///
    /// This widget generates a view over a list of data items via the
    /// [`ListData`] trait. "View widgets" are constructed via a [`Driver`] to
    /// represent visible data items. These view widgets are reassigned as
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
    #[derive(Clone, Debug)]
    #[widget]
    pub struct ListView<A: ListData, V, D = Direction>
    where
        V: Driver<A::Item, A>,
        D: Directional,
    {
        core: widget_core!(),
        frame_offset: Offset,
        frame_size: Size,
        driver: V,
        widgets: Vec<WidgetData<A::Key, V::Widget>>,
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
        selection: LinearSet<A::Key>,
        press_target: Option<(usize, A::Key)>,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a new instance
        pub fn new(driver: V) -> Self {
            Self::new_dir(driver, D::default())
        }
    }
    impl<A: ListData, V: Driver<A::Item, A>> ListView<A, V, kas::dir::Left> {
        /// Construct a new instance
        pub fn left(driver: V) -> Self {
            Self::new(driver)
        }
    }
    impl<A: ListData, V: Driver<A::Item, A>> ListView<A, V, kas::dir::Right> {
        /// Construct a new instance
        pub fn right(driver: V) -> Self {
            Self::new(driver)
        }
    }
    impl<A: ListData, V: Driver<A::Item, A>> ListView<A, V, kas::dir::Up> {
        /// Construct a new instance
        pub fn up(driver: V) -> Self {
            Self::new(driver)
        }
    }
    impl<A: ListData, V: Driver<A::Item, A>> ListView<A, V, kas::dir::Down> {
        /// Construct a new instance
        pub fn down(driver: V) -> Self {
            Self::new(driver)
        }
    }
    impl<A: ListData, V: Driver<A::Item, A>> ListView<A, V, Direction> {
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
        pub fn new_dir(driver: V, direction: D) -> Self {
            ListView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
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
        /// This is not sent to [`Driver::on_messages`].
        ///
        /// The driver may trigger selection by emitting [`Select`] from
        /// [`Driver::on_messages`]. The driver is not notified of selection
        /// except via [`Select`] from view widgets. (TODO: reconsider this.)
        ///
        /// [`Select`]: kas::messages::Select
        pub fn set_selection_mode(&mut self, cx: &mut EventState, mode: SelectionMode) {
            self.sel_mode = mode;
            match mode {
                SelectionMode::None if !self.selection.is_empty() => {
                    self.selection.clear();
                    cx.redraw(self);
                }
                SelectionMode::Single if self.selection.len() > 1 => {
                    if let Some(first) = self.selection.iter().next().cloned() {
                        self.selection.retain(|item| *item == first);
                    }
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
        pub fn selected_iter(&'_ self) -> impl Iterator<Item = &'_ A::Key> + '_ {
            self.selection.iter()
        }

        /// Check whether an entry is selected
        pub fn is_selected(&self, key: &A::Key) -> bool {
            self.selection.contains(key)
        }

        /// Clear all selected items
        pub fn clear_selected(&mut self, cx: &mut EventState) {
            if !self.selection.is_empty() {
                self.selection.clear();
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
        pub fn select(&mut self, cx: &mut EventState, key: A::Key) -> bool {
            match self.sel_mode {
                SelectionMode::None => return false,
                SelectionMode::Single => self.selection.clear(),
                _ => (),
            }
            let r = self.selection.insert(key);
            if r {
                cx.redraw(self);
            }
            r
        }

        /// Directly deselect an item
        ///
        /// Returns `true` if deselected, `false` if not
        /// previously selected or if the key is invalid.
        pub fn deselect(&mut self, cx: &mut EventState, key: &A::Key) -> bool {
            let r = self.selection.remove(key);
            if r {
                cx.redraw(self);
            }
            r
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
        fn update_widgets(&mut self, cx: &mut ConfigCx, data: &A, full: bool) {
            let time = Instant::now();

            let offset = u64::conv(self.scroll_offset().extract(self.direction));
            let mut first_data = usize::conv(offset / u64::conv(self.skip));

            let data_len: usize = self.data_len.cast();
            let cur_len: usize = data_len.min(self.alloc_len.cast());
            first_data = first_data.min(data_len - cur_len);
            self.cur_len = cur_len.cast();
            debug_assert!(self.num_children() <= self.widgets.len());
            self.first_data = first_data.cast();

            let solver = self.position_solver();
            let keys = data.iter_from(solver.first_data, solver.cur_len);

            let mut count = 0;
            for (i, key) in keys.enumerate() {
                count += 1;
                let i = solver.first_data + i;
                let id = key.make_id(self.id_ref());
                let w = &mut self.widgets[i % solver.cur_len];
                if w.key.as_ref() != Some(&key) {
                    self.driver.set_key(&mut w.widget, &key);

                    if let Some(item) = data.borrow(&key) {
                        cx.configure(w.widget.as_node(item.borrow()), id);

                        solve_size_rules(
                            &mut w.widget,
                            cx.size_cx(),
                            Some(self.child_size.0),
                            Some(self.child_size.1),
                        );
                        w.key = Some(key);
                    } else {
                        w.key = None; // disables drawing and clicking
                    }
                } else if full {
                    if let Some(item) = data.borrow(&key) {
                        cx.update(w.widget.as_node(item.borrow()));
                    }
                }
                w.widget.set_rect(cx, solver.rect(i), self.align_hints);
            }

            if count < solver.cur_len {
                log::warn!(
                    "{}: data.iter_from({}, {}) yielded insufficient items (possibly incorrect data.len())", self.identify(),
                    solver.first_data,
                    solver.cur_len,
                );
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
            let child_size = (inner_size / self.ideal_visible)
                .min(self.child_size_ideal)
                .max(self.child_size_min);
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
                        .min(self.child_size_ideal)
                        .max(self.child_size_min);
                }
                size
            });
            axis = AxisInfo::new(axis.is_vertical(), other);

            self.child_size_min = i32::MAX;
            let mut rules = SizeRules::EMPTY;
            for w in self.widgets.iter_mut() {
                if w.key.is_some() {
                    let child_rules = w.widget.size_rules(sizer.re(), axis);
                    if axis.is_vertical() == self.direction.is_vertical() {
                        self.child_size_min = self.child_size_min.min(child_rules.min_size());
                    }
                    rules = rules.max(child_rules);
                }
            }
            if self.child_size_min == i32::MAX {
                self.child_size_min = 0;
            }

            if axis.is_vertical() == self.direction.is_vertical() {
                self.child_size_ideal = rules.ideal_size();
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
                    .min(self.child_size_ideal)
                    .max(self.child_size_min);
                size = rect.size.0;
                skip = child_size.0 + self.child_inter_margin;
            } else {
                child_size.1 = (child_size.1 / self.ideal_visible)
                    .min(self.child_size_ideal)
                    .max(self.child_size_min);
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
                    "set_rect: allocating widgets (old len = {}, new = {})",
                    avail_widgets,
                    req_widgets
                );
                self.widgets.reserve(req_widgets - avail_widgets);
                let key = A::Key::default();
                for _ in avail_widgets..req_widgets {
                    let widget = self.driver.make(&key);
                    self.widgets.push(WidgetData { key: None, widget });
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
                    w.widget.set_rect(cx, solver.rect(i), self.align_hints);
                }
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            let offset = self.scroll_offset();
            draw.with_clip_region(self.rect(), offset, |mut draw| {
                for child in &self.widgets[..self.cur_len.cast()] {
                    if let Some(ref key) = child.key {
                        if self.selection.contains(key) {
                            draw.selection(child.widget.rect(), self.sel_style);
                        }
                    }
                    child.widget.draw(draw.re());
                }
            });
        }
    }

    impl Tile for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.cur_len.cast()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Tile> {
            self.widgets.get(index).map(|w| w.widget.as_tile())
        }
        fn find_child_index(&self, id: &Id) -> Option<usize> {
            let key = A::Key::reconstruct_key(self.id_ref(), id);
            if key.is_some() {
                let num = self.num_children();
                for (i, w) in self.widgets[..num].iter().enumerate() {
                    if key == w.key {
                        return Some(i);
                    }
                }
            }
            None
        }

        #[inline]
        fn translation(&self) -> Offset {
            self.scroll_offset()
        }

        fn probe(&self, coord: Coord) -> Id {
            let coord = coord + self.scroll.offset();
            for child in &self.widgets[..self.cur_len.cast()] {
                if child.key.is_some() {
                    if let Some(id) = child.widget.try_probe(coord) {
                        return id;
                    }
                }
            }
            self.id()
        }
    }

    impl Events for Self {
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
                let key = A::Key::default();
                self.widgets.resize_with(len, || {
                    WidgetData {
                        key: None,
                        widget: self.driver.make(&key),
                    }
                });
                self.alloc_len = len.cast();
            }

            cx.register_nav_fallback(self.id());
        }

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            let id = self.id();
            for w in &mut self.widgets {
                if let Some(ref key) = w.key {
                    if let Some(item) = data.borrow(&key) {
                        let id = key.make_id(&id);
                        cx.configure(w.widget.as_node(item.borrow()), id);
                    }
                }
            }
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            self.selection.retain(|key| data.contains_key(key));

            let data_len = data.len().cast();
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

        fn handle_event(&mut self, cx: &mut EventCx, data: &A, event: Event) -> IsUsed {
            let is_used = match event {
                Event::Command(cmd, _) => {
                    let last = data.len().wrapping_sub(1);
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
                        cx.next_nav_focus(self.widgets[index].widget.id(), false, FocusSource::Key);
                        Used
                    } else {
                        Unused
                    };
                }
                Event::PressStart { ref press } if
                    press.is_primary() && cx.config().event().mouse_nav_focus() =>
                {
                    if let Some(index) = cx.last_child() {
                        self.press_target = self.widgets[index].key.clone().map(|k| (index, k));
                    }
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if w.key.as_ref().map(|k| k == key).unwrap_or(false) {
                            cx.next_nav_focus(w.widget.id(), false, FocusSource::Pointer);
                        }
                    }

                    // Press may also be grabbed by scroll component (replacing
                    // this). Either way we can select on PressEnd.
                    press.grab(self.id(), kas::event::GrabMode::Click).with_cx(cx)
                }
                Event::PressEnd { ref press, success } if press.is_primary() => {
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if success
                            && !matches!(self.sel_mode, SelectionMode::None)
                            && !self.scroll.is_gliding()
                            && w.key.as_ref().map(|k| k == key).unwrap_or(false)
                            && w.widget.rect().contains(press.coord + self.scroll.offset())
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

            let (moved, used_by_sber) = self
                .scroll
                .scroll_by_event(cx, event, self.id(), self.rect());
            if moved {
                // We may process multiple 'moved' events per frame; TIMER_UPDATE_WIDGETS will only
                // be processed once per frame.
                cx.request_frame_timer(self.id(), TIMER_UPDATE_WIDGETS);
            }
            is_used | used_by_sber
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &A) {
            let key: A::Key;
            if let Some(index) = cx.last_child() {
                let w = &mut self.widgets[index];
                key = match w.key.as_ref() {
                    Some(k) => k.clone(),
                    None => return,
                };

                self.driver.on_messages(cx, &mut w.widget, data, &key);
            } else {
                // Message is from self
                key = match self.press_target.as_ref() {
                    Some((_, k)) => k.clone(),
                    None => return,
                };
            }

            if let Some(kas::messages::Select) = cx.try_pop() {
                match self.sel_mode {
                    SelectionMode::None => (),
                    SelectionMode::Single => {
                        cx.redraw(&self);
                        self.selection.clear();
                        self.selection.insert(key.clone());
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
                    }
                }
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, data: &A, scroll: Scroll) {
            let act = self.scroll.scroll(cx, self.rect(), scroll);
            self.update_widgets(&mut cx.config_cx(), data, false);
            cx.action(self, act);
        }
    }

    // Direct implementation of this trait outside of Kas code is not supported!
    impl Widget for Self {
        type Data = A;

        fn for_child_node(
            &mut self,
            data: &A,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            if let Some(w) = self.widgets.get_mut(index) {
                if let Some(ref key) = w.key {
                    if let Some(item) = data.borrow(key) {
                        closure(w.widget.as_node(item.borrow()));
                    }
                }
            }
        }

        // Non-standard implementation to allow mapping new children
        fn _nav_next(
            &mut self,
            cx: &mut ConfigCx,
            data: &A,
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
                self.as_node(data).for_child(index, |mut node| *out = node._nav_next(cx, focus, advance));
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
                let last_data = data.len() - 1;
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

                let act = self.scroll.self_focus_rect(solver.rect(data_index), self.rect());
                if !act.is_empty() {
                    cx.action(&self, act);
                    self.update_widgets(cx, data, false);
                }

                let index = data_index % usize::conv(self.cur_len);

                let mut opt_id = None;
                let out = &mut opt_id;
                self.as_node(data).for_child(index, |mut node| *out = node._nav_next(cx, focus, advance));
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
