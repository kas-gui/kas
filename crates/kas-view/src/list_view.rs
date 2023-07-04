// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view controller

use super::{driver, Driver, SelectionError, SelectionMode, SelectionMsg};
use kas::event::components::ScrollComponent;
use kas::event::{Command, Scroll};
use kas::layout::{solve_size_rules, AlignHints};
#[allow(unused)] use kas::model::SharedData;
use kas::model::{DataKey, ListData, SharedDataMut};
use kas::prelude::*;
use kas::theme::SelectionStyle;
use kas::NavAdvance;
#[allow(unused)] // doc links
use kas_widgets::ScrollBars;
use linear_map::set::LinearSet;
use std::borrow::Borrow;
use std::time::Instant;

#[derive(Clone, Debug, Default)]
struct WidgetData<K, W> {
    key: Option<K>,
    widget: W,
}

impl_scope! {
    /// View controller for 1D indexable data (list)
    ///
    /// This widget supports a view over a list of shared data items.
    ///
    /// The shared data type `T` must support [`ListData`].
    /// One may use `[T]`, `Vec<T>` or a custom shared data type.
    ///
    /// The driver `V` must implement [`Driver`] over `T`.
    /// The default driver is [`driver::View`]; others are available in the
    /// [`driver`] module or [`Driver`] may be implemented directly.
    ///
    /// This widget is [`Scrollable`], supporting keyboard, wheel and drag
    /// scrolling. You may wish to wrap this widget with [`ScrollBars`].
    ///
    /// Optionally, data items may be selected; see [`Self::set_selection_mode`].
    ///
    /// # Messages
    ///
    /// When a view widget pushes a message, [`Driver::on_message`] is called.
    ///
    /// When selection is enabled and an item is selected or deselected, this
    /// widget emits a [`SelectionMsg`].
    #[derive(Clone, Debug)]
    #[widget]
    pub struct ListView<D: Directional, T: ListData, V: Driver<T::Item, T> = driver::View> {
        core: widget_core!(),
        frame_offset: Offset,
        frame_size: Size,
        driver: V,
        /// Empty widget used for sizing; this must be stored between horiz and vert size rule
        /// calculations for correct line wrapping/layout.
        default_widget: V::Widget,
        data: T,
        data_ver: u64,
        widgets: Vec<WidgetData<T::Key, V::Widget>>,
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
        selection: LinearSet<T::Key>,
        press_target: Option<(usize, T::Key)>,
    }

    impl Self
    where
        D: Default,
        V: Default,
    {
        /// Construct a new instance
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. In other cases, use
        /// [`ListView::new_with_direction`].
        pub fn new(data: T) -> Self {
            Self::new_with_dir_driver(D::default(), <V as Default>::default(), data)
        }
    }
    impl Self
    where
        V: Default,
    {
        /// Construct a new instance with explicit direction
        pub fn new_with_direction(direction: D, data: T) -> Self {
            Self::new_with_dir_driver(direction, <V as Default>::default(), data)
        }
    }
    impl Self
    where
        D: Default,
    {
        /// Construct a new instance with explicit driver
        pub fn new_with_driver(driver: V, data: T) -> Self {
            Self::new_with_dir_driver(D::default(), driver, data)
        }
    }
    impl<T: ListData + 'static, V: Driver<T::Item, T>> ListView<Direction, T, V> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, direction: Direction) -> Action {
            self.direction = direction;
            Action::SET_RECT
        }
    }
    impl Self {
        /// Construct a new instance with explicit direction and driver
        pub fn new_with_dir_driver(direction: D, driver: V, data: T) -> Self {
            let default_widget = driver.make();
            ListView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                driver,
                default_widget,
                data,
                data_ver: 0,
                widgets: Default::default(),
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

        /// Access the stored data
        pub fn data(&self) -> &T {
            &self.data
        }

        /// Mutably access the stored data
        ///
        /// It may be necessary to use [`ListView::update_view`] to update the view of this data.
        pub fn data_mut(&mut self) -> &mut T {
            &mut self.data
        }

        /// Borrow a reference to the shared value at `key`
        pub fn borrow_value(&self, key: &T::Key) -> Option<impl Borrow<T::Item> + '_> {
            self.data.borrow(key)
        }

        /// Get a copy of the shared value at `key`
        pub fn get_value(&self, key: &T::Key) -> Option<T::Item> {
            self.data.get_cloned(key)
        }

        /// Set shared data
        ///
        /// This method updates the shared data, if supported (see
        /// [`SharedDataMut::borrow_mut`]). Other widgets sharing this data
        /// are notified of the update, if data is changed.
        pub fn set_value(&self, mgr: &mut EventMgr, key: &T::Key, data: T::Item)
        where
            T: SharedDataMut,
        {
            self.data.set(mgr, key, data);
        }

        /// Update shared data
        ///
        /// This method updates the shared data, if supported (see
        /// [`SharedDataMut::with_ref_mut`]). Other widgets sharing this data
        /// are notified of the update, if data is changed.
        pub fn update_value<U>(
            &self,
            mgr: &mut EventMgr,
            key: &T::Key,
            f: impl FnOnce(&mut T::Item) -> U,
        ) -> Option<U>
        where
            T: SharedDataMut,
        {
            self.data.with_ref_mut(mgr, key, f)
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
        /// This is not sent to [`Driver::on_message`].
        ///
        /// The driver may trigger selection by emitting [`Select`] from
        /// [`Driver::on_message`]. The driver is not notified of selection
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
        pub fn selected_iter(&'_ self) -> impl Iterator<Item = &'_ T::Key> + '_ {
            self.selection.iter()
        }

        /// Check whether an entry is selected
        pub fn is_selected(&self, key: &T::Key) -> bool {
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
        /// Returns `Action::REDRAW` if newly selected, `Action::empty()` if
        /// already selected. Fails if selection mode does not permit selection
        /// or if the key is invalid.
        pub fn select(&mut self, key: T::Key) -> Result<Action, SelectionError> {
            match self.sel_mode {
                SelectionMode::None => return Err(SelectionError::Disabled),
                SelectionMode::Single => self.selection.clear(),
                _ => (),
            }
            if !self.data.contains_key(&key) {
                return Err(SelectionError::Key);
            }
            match self.selection.insert(key) {
                true => Ok(Action::REDRAW),
                false => Ok(Action::empty()),
            }
        }

        /// Directly deselect an item
        ///
        /// Returns `Action::REDRAW` if deselected, `Action::empty()` if not
        /// previously selected or if the key is invalid.
        pub fn deselect(&mut self, key: &T::Key) -> Action {
            match self.selection.remove(key) {
                true => Action::REDRAW,
                false => Action::empty(),
            }
        }

        /// Manually trigger an update to handle changed data
        pub fn update_view(&mut self, mgr: &mut ConfigMgr) {
            let data = &self.data;
            self.data_ver = data.version();

            self.selection.retain(|key| data.contains_key(key));

            let data_len32 = i32::conv(self.data.len());
            let view_size = self.rect().size - self.frame_size;
            let mut content_size = view_size;
            content_size.set_component(self.direction, (self.skip * data_len32 - self.child_inter_margin).max(0));
            *mgr |= self.scroll.set_sizes(view_size, content_size);

            for w in &mut self.widgets {
                w.key = None;
            }
            self.update_widgets(mgr);

            // Force SET_RECT so that scroll-bar wrappers get updated
            *mgr |= Action::SET_RECT;
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
            let cur_len: usize = self.cur_len.cast();
            let mut first_data: usize = self.first_data.cast();
            let mut skip = Offset::ZERO;
            skip.set_component(self.direction, self.skip);

            let mut pos_start = self.core.rect.pos + self.frame_offset;
            if self.direction.is_reversed() {
                first_data = (self.data.len() - first_data).saturating_sub(cur_len);
                pos_start += skip * i32::conv(self.data.len() - 1);
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

        fn update_widgets(&mut self, mgr: &mut ConfigMgr) {
            let time = Instant::now();

            let offset = u64::conv(self.scroll_offset().extract(self.direction));
            let mut first_data = usize::conv(offset / u64::conv(self.skip));

            let mut cur_len = self.widgets.len();
            if self.data.len() - first_data < cur_len {
                cur_len = cur_len.min(self.data.len());
                first_data = self.data.len() - cur_len;
            }
            self.cur_len = cur_len.cast();
            self.first_data = first_data.cast();

            let solver = self.position_solver();
            let keys = self.data.iter_from(solver.first_data, solver.cur_len);

            let mut count = 0;
            for (i, key) in keys.enumerate() {
                count += 1;
                let i = solver.first_data + i;
                let id = key.make_id(self.id_ref());
                let w = &mut self.widgets[i % solver.cur_len];
                if w.key.as_ref() != Some(&key) {
                    // Reset widgets to ensure input state such as cursor
                    // position does not bleed over to next data entry
                    w.widget = self.driver.make();
                    mgr.configure(&mut w.widget, id);

                    if let Some(item) = self.data.borrow(&key) {
                        *mgr |= self.driver.set(&mut w.widget, &key, item.borrow());
                        solve_size_rules(
                            &mut w.widget,
                            mgr.size_mgr(),
                            Some(self.child_size.0),
                            Some(self.child_size.1),
                            self.align_hints.horiz,
                            self.align_hints.vert,
                        );
                        w.key = Some(key);
                    } else {
                        w.key = None; // disables drawing and clicking
                    }
                }
                w.widget.set_rect(mgr, solver.rect(i));
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
            let item_min = self.child_size_min + self.child_inter_margin;
            let num = i32::conv(self.data.len());
            let min_size = (item_min * num - self.child_inter_margin).max(0);
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
        fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset {
            *mgr |= self.scroll.set_offset(offset);
            mgr.config_mgr(|mgr| self.update_widgets(mgr));
            self.scroll.offset()
        }
    }

    #[allow(clippy::manual_clamp)]
    impl Layout for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.cur_len.cast()
        }
        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            let key = T::Key::reconstruct_key(self.id_ref(), id);
            if key.is_some() {
                self.widgets
                    .iter()
                    .enumerate()
                    .filter_map(|(i, w)| (key == w.key).then_some(i))
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
                for _ in avail_widgets..req_widgets {
                    let widget = self.driver.make();
                    self.widgets.push(WidgetData { key: None, widget });
                }
            }
            if req_widgets + 64 <= avail_widgets {
                // Free memory (rarely useful?)
                self.widgets.truncate(req_widgets);
            }

            // Widgets need configuring and updating: do so by re-configuring self.
            mgr.request_reconfigure(self.id());
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
                    if let Some(ref key) = child.key {
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
        type Data = ();

        fn configure(&mut self, mgr: &mut ConfigMgr) {
            if self.widgets.is_empty() {
                // Initial configure: ensure some widgets are loaded to allow
                // better sizing of self.
                self.skip = 1; // hack: avoid div by 0

                let len = self.ideal_visible.cast();
                self.widgets.resize_with(len, || {
                    let key = None;
                    let widget = self.driver.make();
                    WidgetData { key, widget }
                });

                self.update_view(mgr);
            } else {
                // This method is invoked from set_rect to update widgets
                self.update_widgets(mgr);
            }

            mgr.register_nav_fallback(self.id());
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            let response = match event {
                Event::Update { .. } => {
                    let data_ver = self.data.version();
                    if data_ver > self.data_ver {
                        // TODO(opt): use the update payload to indicate which widgets need updating?
                        mgr.config_mgr(|mgr| self.update_view(mgr));
                    }
                    return Response::Used;
                }
                Event::Command(cmd) => {
                    let last = self.data.len().wrapping_sub(1);
                    if last == usize::MAX {
                        return Response::Unused;
                    }

                    let solver = self.position_solver();
                    let cur = match mgr.nav_focus().and_then(|id| self.find_child_index(id)) {
                        Some(index) => solver.child_to_data(index),
                        None => return Response::Unused,
                    };
                    let is_vert = self.direction.is_vertical();
                    let len = solver.cur_len;

                    use Command as C;
                    let data = match cmd {
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
                    return if let Some(i_data) = data {
                        // Set nav focus to i_data and update scroll position
                        if self.scroll.focus_rect(mgr, solver.rect(i_data), self.core.rect) {
                            mgr.config_mgr(|mgr| self.update_widgets(mgr));
                        }
                        let index = i_data % usize::conv(self.cur_len);
                        mgr.next_nav_focus(self.widgets[index].widget.id(), false, true);
                        Response::Used
                    } else {
                        Response::Unused
                    };
                }
                Event::PressStart { ref press } if press.is_primary() && mgr.config().mouse_nav_focus() => {
                    if let Some(index) = mgr.last_child() {
                        self.press_target = self.widgets[index].key.clone().map(|k| (index, k));
                    }
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if w.key.as_ref().map(|k| k == key).unwrap_or(false) {
                            mgr.next_nav_focus(w.widget.id(), false, false);
                        }
                    }

                    // Press may also be grabbed by scroll component (replacing
                    // this). Either way we can select on PressEnd.
                    press.grab(self.id()).with_mgr(mgr)
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
                            mgr.push(kas::message::Select);
                        }
                    }
                    Response::Used
                }
                _ => Response::Unused, // fall through to scroll handler
            };

            let (moved, sber_response) = self
                .scroll
                .scroll_by_event(mgr, event, self.id(), self.core.rect);
            if moved {
                mgr.config_mgr(|mgr| self.update_widgets(mgr));
            }
            response | sber_response
        }

        fn handle_message(&mut self, mgr: &mut EventMgr) {
            let key;
            if let Some(index) = mgr.last_child() {
                let w = &mut self.widgets[index];
                key = match w.key.clone() {
                    Some(k) => k,
                    None => return,
                };

                self.driver.on_message(mgr, &mut w.widget, &self.data, &key);
            } else {
                // Message is from self
                key = match self.press_target.clone() {
                    Some((_, k)) => k,
                    None => return,
                };
            }

            if let Some(kas::message::Select) = mgr.try_pop() {
                match self.sel_mode {
                    SelectionMode::None => (),
                    SelectionMode::Single => {
                        mgr.redraw(self.id());
                        self.selection.clear();
                        self.selection.insert(key.clone());
                        mgr.push(SelectionMsg::Select(key));
                    }
                    SelectionMode::Multiple => {
                        mgr.redraw(self.id());
                        if self.selection.remove(&key) {
                            mgr.push(SelectionMsg::Deselect(key));
                        } else {
                            self.selection.insert(key.clone());
                            mgr.push(SelectionMsg::Select(key));
                        }
                    }
                }
            }
        }

        fn handle_scroll(&mut self, mgr: &mut EventMgr, scroll: Scroll) {
            self.scroll.scroll(mgr, self.rect(), scroll);
            mgr.config_mgr(|mgr| self.update_widgets(mgr));
        }
    }

    // Direct implementation of this trait outside of Kas code is not supported!
    impl Widget for Self {
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn Widget> {
            self.widgets.get(index).and_then(|w| {
                w.key.is_some().then(|| w.widget.as_node())
            })
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
            self.widgets.get_mut(index).and_then(|w| {
                w.key.is_some().then(|| w.widget.as_node_mut())
            })
        }

        // Non-standard behaviour: do not configure children
        fn _configure(&mut self, cx: &mut ConfigMgr, id: WidgetId) {
            self.pre_configure(cx, id);
            self.configure(cx);
        }

        fn _broadcast(&mut self, cx: &mut EventMgr, count: &mut usize, event: Event) {
            kas::impls::_broadcast(self, cx, count, event);
        }

        fn _send(
            &mut self,
            cx: &mut EventMgr,
            id: WidgetId,
            disabled: bool,
            event: Event,
        ) -> Response {
            kas::impls::_send(self, cx, id, disabled, event)
        }

        fn _replay(&mut self, cx: &mut EventMgr, id: WidgetId, msg: kas::Erased) {
            kas::impls::_replay(self, cx, id, msg);
        }

        // Non-standard implementation to allow mapping new children
        fn _nav_next(
            &mut self,
            cx: &mut EventMgr,
            focus: Option<&WidgetId>,
            advance: NavAdvance,
        ) -> Option<WidgetId> {
            if cx.is_disabled(self.id_ref()) || self.cur_len == 0 {
                return None;
            }

            let mut child = focus.and_then(|id| self.find_child_index(id));

            if let Some(index) = child {
                if let Some(id) = self
                    .get_child_mut(index)
                    .and_then(|w| w._nav_next(cx, focus, advance))
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
                let last_data = self.data.len() - 1;
                let data = if let Some(index) = child {
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

                if self.scroll.focus_rect(cx, solver.rect(data), self.core.rect) {
                    cx.config_mgr(|mgr| self.update_widgets(mgr));
                }

                let index = data % usize::conv(self.cur_len);
                if let Some(id) = self
                    .get_child_mut(index)
                    .and_then(|w| w._nav_next(cx, focus, advance))
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
