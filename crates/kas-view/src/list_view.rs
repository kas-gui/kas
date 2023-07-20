// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view controller

use crate::{DataKey, ListData, SelectionMode, SelectionMsg};
use kas::event::components::ScrollComponent;
use kas::event::{Command, Scroll};
use kas::layout::{solve_size_rules, AlignHints};
use kas::prelude::*;
use kas::theme::SelectionStyle;
use kas::NavAdvance;
#[allow(unused)] // doc links
use kas_widgets::ScrollBars;
use linear_map::set::LinearSet;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::time::Instant;

/// View widget guard
///
/// The guard can:
///
/// -   construct (empty) widgets with [`Self::make`]
/// -   assign data to an existing widget with [`Self::set`]
/// -   (optional) handle messages from a widget with [`Self::on_messages`]
pub trait ListViewGuard<A: ListData> {
    /// Type of the "view widget" used for each data item
    ///
    /// NOTE: with RPITIT this should become an alias to `Self::make()`.
    type Widget: kas::Widget<Data = A::Item>;

    /// Construct a new view widget
    fn make(&mut self, key: &A::Key) -> Self::Widget;

    // /// Reset and reassign a widget
    // ///
    // /// This method is called when a view widget is assigned a different key.
    // fn reset(&mut self, widget: &mut Self::Widget, key: &A::Key);

    /// Called to bind an existing view widget to a new key
    ///
    /// This should reset widget metadata, for example so that when a view
    /// widget with a text selection is assigned to a new key it does not
    /// attempt to apply the old selection to the new text.
    ///
    /// The default implementation simply replaces widget with `self.make(key)`,
    /// which is sufficient, if not always optimal.
    ///
    /// This does not need to set data; [`Widget::update`] does that.
    fn set_key(&mut self, widget: &mut Self::Widget, key: &A::Key) {
        *widget = self.make(key);
    }

    /// Handle a message from a widget
    ///
    /// This method is called when a view widget returns with a message; it
    /// may retrieve this message with [`EventMgr::try_pop`].
    ///
    /// There are three main ways of implementing this method:
    ///
    /// 1.  Do nothing. This is always safe, though may result in unhandled
    ///     message warnings when the view widget is interactive.
    /// 2.  On user input actions, view widgets send a message including their
    ///     content (potentially wrapped with a user-defined enum or struct
    ///     type). The implementation of this method retrieves this message and
    ///     updates `data` given this content. In this case, the `widget`
    ///     parameter is not used.
    /// 3.  On user input actions, view widgets send a "trigger" message (likely
    ///     a unit struct). The implementation of this method retrieves this
    ///     message and updates `data` using values read from `widget`.
    ///
    /// See, for example, the implementation for [`CheckButton`]: the `make`
    /// method assigns a state-change handler which `on_messages` uses to update
    /// the shared data.
    ///
    /// Default implementation: do nothing.
    fn on_messages(
        &mut self,
        cx: &mut EventMgr,
        widget: &mut Self::Widget,
        data: &A,
        key: &A::Key,
    ) {
        let _ = (widget, data, key, cx);
    }
}

#[derive(Clone, Debug, Default)]
struct WidgetData<K, V, W> {
    key: Option<(K, V)>,
    widget: W,
}

impl_scope! {
    /// View controller for 1D indexable data (list)
    ///
    /// This widget generates a view over a list of data items, generating or
    /// remapping "view widgets" for data items as required to cover the view
    /// area. View widgets are only guaranteed to maintain state while visible.
    ///
    /// [`ListData::iter_from`] is used to generate a list of data items which
    /// are then geneverated via [`ListViewGuard::make`].
    ///
    /// When the view is scrolled and when [`Widget::update`] is called on the
    /// `ListView`, for any key which differs from the cached view widget that
    /// view widget is re-assigned via [`ListViewGuard::set_key`], otherwise (on
    /// update) the view widget receives [`Widget::update`].
    ///
    /// This widget is [`Scrollable`], supporting keyboard, wheel and drag
    /// scrolling. You may wish to wrap this widget with [`ScrollBars`].
    ///
    /// View widgets handle events like normal. To associate a returned message
    /// with a data key, either embed that key in the message while constructing
    /// the widget with [`ListViewGuard::make`] or intercept the message in
    /// [`ListViewGuard::on_messages`].
    ///
    /// # Selection
    ///
    /// Item selection must be enabled explicitly via
    /// [`Self::set_selection_mode`] or [`Self::with_selection_mode`].
    ///
    /// If enabled, a view widget may emit [`kas::message::Select`]
    /// to cause itself to be selected. All changes to selection are reported
    /// by this widget emitting a [`SelectionMsg`].
    #[derive(Clone, Debug)]
    #[widget]
    pub struct ListView<A: ListData, G: ListViewGuard<A>, D: Directional> {
        core: widget_core!(),
        frame_offset: Offset,
        frame_size: Size,
        guard: G,
        /// Empty widget used for sizing; this must be stored between horiz and vert size rule
        /// calculations for correct line wrapping/layout.
        default_widget: G::Widget,
        widgets: Vec<WidgetData<A::Key, A::Version, G::Widget>>,
        data_len: u32,
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
        pub fn new(guard: G) -> Self {
            Self::new_with_direction(D::default(), guard)
        }
    }
    impl<A: ListData, G: ListViewGuard<A>> ListView<A, G, Direction> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, direction: Direction) -> Action {
            self.direction = direction;
            Action::SET_RECT
        }
    }
    impl Self {
        /// Construct a new instance with explicit direction and guard
        pub fn new_with_direction(direction: D, mut guard: G) -> Self {
            let default_widget = guard.make(&A::Key::default());
            ListView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                guard,
                default_widget,
                widgets: Default::default(),
                data_len: 0,
                cur_len: 0,
                first_data: 0,
                direction,
                align_hints: Default::default(),
                ideal_visible: 5,
                child_size_min: 0,
                child_size_ideal: 0,
                child_inter_margin: 0,
                skip: 0,
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
        /// This is not sent to [`ListViewGuard::on_messages`].
        ///
        /// The guard may trigger selection by emitting [`Select`] from
        /// [`ListViewGuard::on_messages`]. The guard is not notified of selection
        /// except via [`Select`] from view widgets. (TODO: reconsider this.)
        ///
        /// [`Select`]: kas::message::Select
        pub fn set_selection_mode(&mut self, mode: SelectionMode) -> Action {
            self.sel_mode = mode;
            match mode {
                SelectionMode::None if !self.selection.is_empty() => {
                    self.selection.clear();
                    Action::REDRAW
                }
                SelectionMode::Single if self.selection.len() > 1 => {
                    if let Some(first) = self.selection.iter().next().cloned() {
                        self.selection.retain(|item| *item == first);
                    }
                    Action::REDRAW
                }
                _ => Action::empty(),
            }
        }
        /// Set the selection mode (inline)
        ///
        /// See [`Self::set_selection_mode`] documentation.
        #[must_use]
        pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
            let _ = self.set_selection_mode(mode);
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
        pub fn set_selection_style(&mut self, style: SelectionStyle) -> Action {
            let action = if style.is_external() != self.sel_style.is_external() {
                Action::RESIZE
            } else {
                Action::empty()
            };
            self.sel_style = style;
            action
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
        pub fn clear_selected(&mut self) -> Action {
            if self.selection.is_empty() {
                Action::empty()
            } else {
                self.selection.clear();
                Action::REDRAW
            }
        }

        /// Directly select an item
        ///
        /// Does nothing if [`Self::selection_mode`] is [`SelectionMode::None`].
        /// Does not verify the validity of `key`.
        /// Does not send [`SelectionMsg`] messages.
        ///
        /// Returns `Action::REDRAW` if newly selected, `Action::empty()` if
        /// already selected. Fails if selection mode does not permit selection
        /// or if the key is invalid.
        pub fn select(&mut self, key: A::Key) -> Action {
            match self.sel_mode {
                SelectionMode::None => return Action::empty(),
                SelectionMode::Single => self.selection.clear(),
                _ => (),
            }
            match self.selection.insert(key) {
                true => Action::REDRAW,
                false => Action::empty(),
            }
        }

        /// Directly deselect an item
        ///
        /// Returns `Action::REDRAW` if deselected, `Action::empty()` if not
        /// previously selected or if the key is invalid.
        pub fn deselect(&mut self, key: &A::Key) -> Action {
            match self.selection.remove(key) {
                true => Action::REDRAW,
                false => Action::empty(),
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

            let mut pos_start = self.core.rect.pos + self.frame_offset;
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

        fn update_widgets(&mut self, data: &A, cx: &mut ConfigMgr) {
            let time = Instant::now();

            let offset = u64::conv(self.scroll_offset().extract(self.direction));
            let mut first_data = usize::conv(offset / u64::conv(self.skip));

            let data_len: usize = self.data_len.cast();
            let mut cur_len: usize = self.widgets.len();
            if data_len - first_data < cur_len {
                cur_len = cur_len.min(data_len);
                first_data = data_len - cur_len;
            }
            self.cur_len = cur_len.cast();
            self.first_data = first_data.cast();

            let solver = self.position_solver();
            let keys = data.iter_from(solver.first_data, solver.cur_len);

            let mut count = 0;
            for (i, key_ver) in keys.enumerate() {
                count += 1;
                let i = solver.first_data + i;
                let id = key_ver.0.make_id(self.id_ref());
                let w = &mut self.widgets[i % solver.cur_len];
                if w.key.as_ref() != Some(&key_ver) {
                    // Key and/or version changed
                    if w.key.as_ref().map(|kv| &kv.0) != Some(&key_ver.0) {
                        // Key part changed
                        self.guard.set_key(&mut w.widget, &key_ver.0);
                    }

                    if let Some(item) = data.borrow(&key_ver.0) {
                        cx.configure(w.widget.as_node_mut(item.borrow()), id);

                        solve_size_rules(
                            &mut w.widget,
                            cx.size_mgr(),
                            Some(self.child_size.0),
                            Some(self.child_size.1),
                            self.align_hints.horiz,
                            self.align_hints.vert,
                        );
                        w.key = Some(key_ver);
                    } else {
                        w.key = None; // disables drawing and clicking
                    }
                }
                w.widget.set_rect(cx, solver.rect(i));
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
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            // TODO: maybe we should support a scroll bar on the other axis?
            // We would need to report a fake min-child-size to enable scrolling.
            let data_len: i32 = self.data_len.cast();
            let item_min = self.child_size_min + self.child_inter_margin;
            let min_size = (item_min * data_len - self.child_inter_margin).max(0);
            (
                self.direction.is_horizontal() && min_size > size.0,
                self.direction.is_vertical() && min_size > size.1,
            )
        }

        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.scroll.max_offset()
        }

        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.scroll.offset()
        }

        #[inline]
        fn set_scroll_offset(&mut self, data: &A, cx: &mut EventMgr, offset: Offset) -> Offset {
            *cx |= self.scroll.set_offset(offset);
            cx.config_mgr(|cx| self.update_widgets(data, cx));
            self.scroll.offset()
        }
    }

    impl Layout for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.cur_len.cast()
        }
        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            let key = A::Key::reconstruct_key(self.id_ref(), id);
            if key.is_some() {
                self.widgets
                    .iter()
                    .enumerate()
                    .filter_map(|(i, w)| (key.as_ref() == w.key.as_ref().map(|(k, _)| k)).then_some(i))
                    .next()
            } else {
                None
            }
        }
        #[inline]
        fn make_child_id(&mut self, _: usize) -> WidgetId {
            // We configure children in update_widgets and do not want this method to be called
            unimplemented!()
        }

        fn size_rules(&mut self, size_mgr: SizeMgr, mut axis: AxisInfo) -> SizeRules {
            // We use an invisible frame for highlighting selections, drawing into the margin
            let inner_margin = if self.sel_style.is_external() {
                size_mgr.inner_margins().extract(axis)
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
            axis = AxisInfo::new(axis.is_vertical(), other, axis.align());

            let mut rules = self.default_widget.size_rules(size_mgr.re(), axis);
            if axis.is_vertical() == self.direction.is_vertical() {
                self.child_size_min = rules.min_size();
            }

            if !self.widgets.is_empty() {
                for w in self.widgets.iter_mut() {
                    rules = rules.max(w.widget.size_rules(size_mgr.re(), axis));
                }
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
            self.align_hints.set_component(axis, axis.align());
            rules
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            self.core.rect = rect;

            let mut child_size = rect.size - self.frame_size;
            let skip;
            let req_widgets;
            if self.direction.is_horizontal() {
                child_size.0 = (child_size.0 / self.ideal_visible)
                    .min(self.child_size_ideal)
                    .max(self.child_size_min);
                skip = child_size.0 + self.child_inter_margin;
                req_widgets = usize::conv((rect.size.0 + skip - 1) / skip + 1);
            } else {
                child_size.1 = (child_size.1 / self.ideal_visible)
                    .min(self.child_size_ideal)
                    .max(self.child_size_min);
                skip = child_size.1 + self.child_inter_margin;
                req_widgets = usize::conv((rect.size.1 + skip - 1) / skip + 1);
            }

            self.child_size = child_size;
            self.skip = skip;

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
                    let widget = self.guard.make(&key);
                    self.widgets.push(WidgetData { key: None, widget });
                }
            }
            if req_widgets + 64 <= avail_widgets {
                // Free memory (rarely useful?)
                self.widgets.truncate(req_widgets);
            }

            // Widgets need configuring and updating: do so by updating self.
            mgr.request_update(self.id());
        }

        #[inline]
        fn translation(&self) -> Offset {
            self.scroll_offset()
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }

            let coord = coord + self.scroll.offset();
            for child in &mut self.widgets[..self.cur_len.cast()] {
                if let Some(id) = child.widget.find_id(coord) {
                    return Some(id);
                }
            }
            Some(self.id())
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let offset = self.scroll_offset();
            draw.with_clip_region(self.core.rect, offset, |mut draw| {
                for child in &mut self.widgets[..self.cur_len.cast()] {
                    if let Some((ref key, _)) = child.key {
                        if self.selection.contains(key) {
                            draw.selection(child.widget.rect(), self.sel_style);
                        }
                    }
                    draw.recurse(&mut child.widget);
                }
            });
        }
    }

    impl Events for Self {
        fn configure(&mut self, _: &A, cx: &mut ConfigMgr) {
            if self.widgets.is_empty() {
                // Initial configure: ensure some widgets are loaded to allow
                // better sizing of self.
                self.skip = 1; // hack: avoid div by 0

                let len = self.ideal_visible.cast();
                let key = A::Key::default();
                self.widgets.resize_with(len, || {
                    WidgetData {
                        key: None,
                        widget: self.guard.make(&key),
                    }
                });
            }

            cx.register_nav_fallback(self.id());
        }

        fn update(&mut self, data: &A, cx: &mut ConfigMgr) {
            self.selection.retain(|key| data.contains_key(key));

            self.data_len = data.len().cast();
            let data_len: i32 = self.data_len.cast();
            let view_size = self.rect().size - self.frame_size;
            let mut content_size = view_size;
            content_size.set_component(
                self.direction,
                (self.skip * data_len - self.child_inter_margin).max(0),
            );
            *cx |= self.scroll.set_sizes(view_size, content_size);

            self.update_widgets(data, cx);
        }

        fn handle_event(&mut self, data: &A, cx: &mut EventMgr, event: Event) -> Response {
            let response = match event {
                Event::Command(cmd) => {
                    let last = data.len().wrapping_sub(1);
                    if last == usize::MAX {
                        return Response::Unused;
                    }

                    let solver = self.position_solver();
                    let cur = match cx.nav_focus().and_then(|id| self.find_child_index(id)) {
                        Some(index) => solver.child_to_data(index),
                        None => return Response::Unused,
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
                        if self.scroll.focus_rect(cx, solver.rect(i_data), self.core.rect) {
                            cx.config_mgr(|cx| self.update_widgets(data, cx));
                        }
                        let index = i_data % usize::conv(self.cur_len);
                        cx.next_nav_focus(self.widgets[index].widget.id(), false, true);
                        Response::Used
                    } else {
                        Response::Unused
                    };
                }
                Event::PressStart { ref press } if press.is_primary() && cx.config().mouse_nav_focus() => {
                    if let Some(index) = cx.last_child() {
                        self.press_target = self.widgets[index].key.clone().map(|(k, _)| (index, k));
                    }
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if w.key.as_ref().map(|(k, _)| k == key).unwrap_or(false) {
                            cx.next_nav_focus(w.widget.id(), false, false);
                        }
                    }

                    // Press may also be grabbed by scroll component (replacing
                    // this). Either way we can select on PressEnd.
                    press.grab(self.id()).with_mgr(cx)
                }
                Event::PressEnd { ref press, success } if press.is_primary() => {
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if success
                            && !matches!(self.sel_mode, SelectionMode::None)
                            && !self.scroll.is_gliding()
                            && w.key.as_ref().map(|(k, _)| k == key).unwrap_or(false)
                            && w.widget.rect().contains(press.coord + self.scroll.offset())
                        {
                            cx.push(kas::message::Select);
                        }
                    }
                    Response::Used
                }
                _ => Response::Unused, // fall through to scroll handler
            };

            let (moved, sber_response) = self
                .scroll
                .scroll_by_event(cx, event, self.id(), self.core.rect);
            if moved {
                cx.config_mgr(|cx| self.update_widgets(data, cx));
            }
            response | sber_response
        }

        fn handle_messages(&mut self, data: &A, cx: &mut EventMgr) {
            let key;
            if let Some(index) = cx.last_child() {
                let w = &mut self.widgets[index];
                key = match w.key.clone() {
                    Some((k, _)) => k,
                    None => return,
                };

                self.guard.on_messages(cx, &mut w.widget, data, &key);
            } else {
                // Message is from self
                key = match self.press_target.clone() {
                    Some((_, k)) => k,
                    None => return,
                };
            }

            if let Some(kas::message::Select) = cx.try_pop() {
                match self.sel_mode {
                    SelectionMode::None => (),
                    SelectionMode::Single => {
                        cx.redraw(self.id());
                        self.selection.clear();
                        self.selection.insert(key.clone());
                        cx.push(SelectionMsg::Select(key));
                    }
                    SelectionMode::Multiple => {
                        cx.redraw(self.id());
                        if self.selection.remove(&key) {
                            cx.push(SelectionMsg::Deselect(key));
                        } else {
                            self.selection.insert(key.clone());
                            cx.push(SelectionMsg::Select(key));
                        }
                    }
                }
            }
        }

        fn handle_scroll(&mut self, data: &A, cx: &mut EventMgr, scroll: Scroll) {
            self.scroll.scroll(cx, self.rect(), scroll);
            cx.config_mgr(|cx| self.update_widgets(data, cx));
        }
    }

    // Direct implementation of this trait outside of Kas code is not supported!
    impl Widget for Self {
        type Data = A;

        fn for_child_impl(
            &self,
            data: &A,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            if let Some(w) = self.widgets.get(index) {
                if let Some((ref key, _)) = w.key {
                    if let Some(item) = data.borrow(key) {
                        closure(w.widget.as_node(item.borrow()));
                    }
                }
            }
        }
        fn for_child_mut_impl(
            &mut self,
            data: &A,
            index: usize,
            closure: Box<dyn FnOnce(NodeMut<'_>) + '_>,
        ) {
            if let Some(w) = self.widgets.get_mut(index) {
                if let Some((ref key, _)) = w.key {
                    if let Some(item) = data.borrow(key) {
                        closure(w.widget.as_node_mut(item.borrow()));
                    }
                }
            }
        }

        // Non-standard behaviour: do not configure children
        fn _configure(&mut self, data: &A, cx: &mut ConfigMgr, id: WidgetId) {
            self.pre_configure(cx, id);
            self.configure(data, cx);
        }

        fn _update(&mut self, data: &A, cx: &mut ConfigMgr) {
            self.update(data, cx);
        }

        fn _broadcast(&mut self, data: &A, cx: &mut EventMgr, count: &mut usize, event: Event) {
            kas::impls::_broadcast(self, data, cx, count, event);
        }

        fn _send(
            &mut self,
            data: &A,
            cx: &mut EventMgr,
            id: WidgetId,
            disabled: bool,
            event: Event,
        ) -> Response {
            kas::impls::_send(self, data, cx, id, disabled, event)
        }

        fn _replay(&mut self, data: &A, cx: &mut EventMgr, id: WidgetId, msg: kas::Erased) {
            kas::impls::_replay(self, data, cx, id, msg);
        }

        // Non-standard implementation to allow mapping new children
        fn _nav_next(
            &mut self,
            data: &A,
            cx: &mut EventMgr,
            focus: Option<&WidgetId>,
            advance: NavAdvance,
        ) -> Option<WidgetId> {
            if cx.is_disabled(self.id_ref()) || self.cur_len == 0 {
                return None;
            }

            let mut child = focus.and_then(|id| self.find_child_index(id));

            if let Some(index) = child {
                if let Some(Some(id)) = self.as_node_mut(data)
                    .for_child(index, |mut w| w._nav_next(cx, focus, advance))
                {
                    return Some(id);
                }
            }

            let reverse = match advance {
                NavAdvance::None => return None,
                NavAdvance::Forward(_) => false,
                NavAdvance::Reverse(_) => true,
            };

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

                if self.scroll.focus_rect(cx, solver.rect(data_index), self.core.rect) {
                    cx.config_mgr(|mgr| self.update_widgets(data, mgr));
                }

                let index = data_index % usize::conv(self.cur_len);
                if let Some(Some(id)) = self.as_node_mut(data)
                    .for_child(index, |mut w| w._nav_next(cx, focus, advance))
                {
                    return Some(id);
                }

                child = Some(index);
            }
        }
    }
}

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
