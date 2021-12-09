// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view widget

use super::{driver, Driver, PressPhase, SelectionError, SelectionMode};
#[allow(unused)] // doc links
use crate::ScrollBars;
use crate::{ScrollComponent, Scrollable};
use kas::event::{ChildMsg, Command, CursorIcon, GrabMode, PressSource};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::updatable::{ListData, UpdatableHandler};
use linear_map::set::LinearSet;
use log::{debug, trace};
use std::time::Instant;
use UpdatableHandler as UpdHandler;

#[derive(Clone, Debug, Default)]
struct WidgetData<K, W> {
    key: Option<K>,
    widget: W,
}

widget! {
    /// List view widget
    ///
    /// This widget supports a view over a list of shared data items.
    ///
    /// The shared data type `T` must support [`ListData`] and
    /// [`UpdatableHandler`], the latter with key type `T::Key` and message type
    /// matching the widget's message. One may use [`kas::updatable::SharedRc`]
    /// or a custom shared data type.
    ///
    /// The driver `V` must implement [`Driver`], with data type
    /// `<T as ListData>::Item`. Several implementations are available in the
    /// [`driver`] module or a custom implementation may be used.
    ///
    /// This widget is [`Scrollable`], supporting keyboard, wheel and drag
    /// scrolling. You may wish to wrap this widget with [`ScrollBars`].
    #[derive(Clone, Debug)]
    #[handler(msg=ChildMsg<T::Key, <V::Widget as Handler>::Msg>)]
    pub struct ListView<
        D: Directional,
        T: ListData + UpdHandler<T::Key, V::Msg> + 'static,
        V: Driver<T::Item> = driver::Default,
    > {
        #[widget_core]
        core: CoreData,
        frame_offset: Offset,
        frame_size: Size,
        view: V,
        data: T,
        widgets: Vec<WidgetData<T::Key, V::Widget>>,
        /// The number of widgets in use (cur_len ≤ widgets.len())
        cur_len: u32,
        /// The first visible data item
        direction: D,
        align_hints: AlignHints,
        ideal_visible: i32,
        child_size_min: i32,
        child_size_ideal: i32,
        child_inter_margin: i32,
        child_size: Size,
        scroll: ScrollComponent,
        sel_mode: SelectionMode,
        // TODO(opt): replace selection list with RangeOrSet type?
        selection: LinearSet<T::Key>,
        press_event: Option<PressSource>,
        press_phase: PressPhase,
        press_target: Option<T::Key>,
    }

    impl Self where D: Default, V: Default {
        /// Construct a new instance
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. In other cases, use
        /// [`ListView::new_with_direction`].
        pub fn new(data: T) -> Self {
            Self::new_with_dir_driver(D::default(), <V as Default>::default(), data)
        }
    }
    impl Self where V: Default {
        /// Construct a new instance with explicit direction
        pub fn new_with_direction(direction: D, data: T) -> Self {
            Self::new_with_dir_driver(direction, <V as Default>::default(), data)
        }
    }
    impl Self where D: Default {
        /// Construct a new instance with explicit view
        pub fn new_with_driver(view: V, data: T) -> Self {
            Self::new_with_dir_driver(D::default(), view, data)
        }
    }
    impl<T: ListData + UpdHandler<T::Key, V::Msg>, V: Driver<T::Item>> ListView<Direction, T, V> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, direction: Direction) -> TkAction {
            self.direction = direction;
            TkAction::SET_SIZE
        }
    }
    impl Self {
        /// Construct a new instance with explicit direction and view
        pub fn new_with_dir_driver(direction: D, view: V, data: T) -> Self {
            ListView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                view,
                data,
                widgets: Default::default(),
                cur_len: 0,
                direction,
                align_hints: Default::default(),
                ideal_visible: 5,
                child_size_min: 0,
                child_size_ideal: 0,
                child_inter_margin: 0,
                child_size: Size::ZERO,
                scroll: Default::default(),
                sel_mode: SelectionMode::None,
                selection: Default::default(),
                press_event: None,
                press_phase: PressPhase::None,
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

        /// Get a copy of the shared value at `key`
        pub fn get_value(&self, key: &T::Key) -> Option<T::Item> {
            self.data.get_cloned(key)
        }

        /// Set shared data
        ///
        /// This method updates the shared data, if supported (see
        /// [`ListData::update`]). Other widgets sharing this data are notified
        /// of the update, if data is changed.
        pub fn set_value(&self, mgr: &mut Manager, key: &T::Key, data: T::Item) {
            if let Some(handle) = self.data.update(key, data) {
                mgr.trigger_update(handle, 0);
            }
        }

        /// Update shared data
        ///
        /// This is purely a convenience method over [`ListView::set_value`].
        /// It does nothing if no value is found at `key`.
        /// It notifies other widgets of updates to the shared data.
        pub fn update_value<F: Fn(T::Item) -> T::Item>(&self, mgr: &mut Manager, key: &T::Key, f: F) {
            if let Some(item) = self.get_value(key) {
                self.set_value(mgr, key, f(item));
            }
        }

        /// Get the current selection mode
        pub fn selection_mode(&self) -> SelectionMode {
            self.sel_mode
        }
        /// Set the current selection mode
        pub fn set_selection_mode(&mut self, mode: SelectionMode) -> TkAction {
            self.sel_mode = mode;
            match mode {
                SelectionMode::None if !self.selection.is_empty() => {
                    self.selection.clear();
                    TkAction::REDRAW
                }
                SelectionMode::Single if self.selection.len() > 1 => {
                    if let Some(first) = self.selection.iter().next().cloned() {
                        self.selection.retain(|item| *item == first);
                    }
                    TkAction::REDRAW
                }
                _ => TkAction::empty(),
            }
        }
        /// Set the selection mode (inline)
        pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
            let _ = self.set_selection_mode(mode);
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
        ///
        /// Does not send [`ChildMsg`] responses.
        pub fn clear_selected(&mut self) {
            self.selection.clear();
        }

        /// Directly select an item
        ///
        /// Returns `true` if selected, `false` if already selected.
        /// Fails if selection mode does not permit selection or if the key is
        /// invalid.
        ///
        /// Does not send [`ChildMsg`] responses.
        pub fn select(&mut self, key: T::Key) -> Result<bool, SelectionError> {
            match self.sel_mode {
                SelectionMode::None => return Err(SelectionError::Disabled),
                SelectionMode::Single => self.selection.clear(),
                _ => (),
            }
            if !self.data.contains_key(&key) {
                return Err(SelectionError::Key);
            }
            Ok(self.selection.insert(key))
        }

        /// Directly deselect an item
        ///
        /// Returns `true` if deselected, `false` if not previously selected.
        /// Also returns `false` on invalid keys.
        ///
        /// Does not send [`ChildMsg`] responses.
        pub fn deselect(&mut self, key: &T::Key) -> bool {
            self.selection.remove(key)
        }

        /// Manually trigger an update to handle changed data
        pub fn update_view(&mut self, mgr: &mut Manager) {
            let data = &self.data;
            self.selection.retain(|key| data.contains_key(key));
            for w in &mut self.widgets {
                w.key = None;
            }
            self.update_widgets(mgr);
            // Force SET_SIZE so that scroll-bar wrappers get updated
            trace!("update_view triggers SET_SIZE");
            *mgr |= TkAction::SET_SIZE;
        }

        /// Get the direction of contents
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Set the preferred number of items visible (inline)
        ///
        /// This affects the (ideal) size request and whether children are sized
        /// according to their ideal or minimum size but not the minimum size.
        pub fn with_num_visible(mut self, number: i32) -> Self {
            self.ideal_visible = number;
            self
        }

        /// Construct a position solver. Note: this does more work and updates to
        /// self than is necessary in several cases where it is used.
        fn position_solver(&mut self, mgr: &mut Manager) -> PositionSolver {
            let data_len = self.data.len();
            let data_len32 = i32::conv(data_len);
            let view_size = self.rect().size;
            let mut content_size = view_size;
            let mut skip;
            if self.direction.is_horizontal() {
                skip = Offset(self.child_size.0 + self.child_inter_margin, 0);
                content_size.0 = (skip.0 * data_len32 - self.child_inter_margin).max(0);
            } else {
                skip = Offset(0, self.child_size.1 + self.child_inter_margin);
                content_size.1 = (skip.1 * data_len32 - self.child_inter_margin).max(0);
            }
            *mgr |= self.scroll.set_sizes(view_size, content_size);

            let offset = u64::conv(self.scroll_offset().extract(self.direction));
            // first visible data item, in downward direction:
            let mut first_data = usize::conv(offset / u64::conv(skip.extract(self.direction)));

            // set_rect allocates enough widgets to view a page; we update widget-data allocations
            let cur_len = self.widgets.len().min(data_len - first_data);
            self.cur_len = cur_len.cast();

            let mut pos_start = self.core.rect.pos + self.frame_offset;
            if self.direction.is_reversed() {
                first_data = (data_len - first_data).saturating_sub(cur_len);
                pos_start += skip * i32::conv(data_len - 1);
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

        fn update_widgets(&mut self, mgr: &mut Manager) {
            let time = Instant::now();
            let solver = self.position_solver(mgr);

            let mut action = TkAction::empty();
            for (i, item) in self
                .data
                .iter_vec_from(solver.first_data, solver.cur_len)
                .into_iter()
                .enumerate()
            {
                let i = solver.first_data + i;
                let key = Some(item.0.clone());
                let w = &mut self.widgets[i % solver.cur_len];
                if key != w.key {
                    w.key = key;
                    action |= self.view.set(&mut w.widget, item.1);
                }
                let rect = solver.rect(i);
                if w.widget.rect() != rect {
                    w.widget.set_rect(mgr, rect, self.align_hints);
                }
            }
            *mgr |= action;
            let dur = (Instant::now() - time).as_micros();
            trace!("ListView::update_widgets completed in {}μs", dur);
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            // TODO: maybe we should support a scrollbar on the other axis?
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
        fn set_scroll_offset(&mut self, mgr: &mut Manager, offset: Offset) -> Offset {
            *mgr |= self.scroll.set_offset(offset);
            self.update_widgets(mgr);
            self.scroll.offset()
        }
    }

    impl WidgetChildren for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn WidgetConfig> {
            self.widgets.get(index).map(|w| w.widget.as_widget())
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
            self.widgets
                .get_mut(index)
                .map(|w| w.widget.as_widget_mut())
        }
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut Manager) {
            if let Some(handle) = self.data.update_handle() {
                mgr.update_on_handle(handle, self.id());
            }
            mgr.register_nav_fallback(self.id());
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
            // We use an invisible frame for highlighting selections, drawing into the margin
            let inner_margin = size_handle.inner_margin().extract(axis);
            let frame = kas::layout::FrameRules::new_sym(0, inner_margin, 0);

            // We use a default-generated widget to generate size rules
            let mut rules = self.view.new().size_rules(size_handle, axis);
            if axis.is_vertical() == self.direction.is_vertical() {
                self.child_size_min = rules.min_size();
                self.child_size_ideal = rules.ideal_size();
                let m = rules.margins_i32();
                self.child_inter_margin = m.0.max(m.1).max(inner_margin);
                rules.multiply_with_margin(2, self.ideal_visible);
                rules.set_stretch(rules.stretch().max(Stretch::High));
            }
            let (rules, offset, size) = frame.surround_with_margin(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, mut align: AlignHints) {
            self.core.rect = rect;

            let mut child_size = rect.size - self.frame_size;
            let num = if self.direction.is_horizontal() {
                if child_size.0 >= self.ideal_visible * self.child_size_ideal {
                    child_size.0 = self.child_size_ideal;
                } else {
                    child_size.0 = self.child_size_min;
                }
                let skip = child_size.0 + self.child_inter_margin;
                align.horiz = None;
                (rect.size.0 + skip - 1) / skip + 1
            } else {
                if child_size.1 >= self.ideal_visible * self.child_size_ideal {
                    child_size.1 = self.child_size_ideal;
                } else {
                    child_size.1 = self.child_size_min;
                }
                let skip = child_size.1 + self.child_inter_margin;
                align.vert = None;
                (rect.size.1 + skip - 1) / skip + 1
            };

            self.child_size = child_size;
            self.align_hints = align;

            let old_num = self.widgets.len();
            let num = usize::conv(num);
            if old_num < num {
                debug!("allocating widgets (old len = {}, new = {})", old_num, num);
                *mgr |= TkAction::RECONFIGURE;
                self.widgets.reserve(num - old_num);
                mgr.size_handle(|size_handle| {
                    for _ in old_num..num {
                        let mut widget = self.view.new();
                        solve_size_rules(
                            &mut widget,
                            size_handle,
                            Some(child_size.0),
                            Some(child_size.1),
                        );
                        self.widgets.push(WidgetData { key: None, widget });
                    }
                });
            } else if num + 64 <= old_num {
                // Free memory (rarely useful?)
                self.widgets.truncate(num);
            }
            self.update_widgets(mgr);
        }

        fn spatial_nav(
            &mut self,
            mgr: &mut Manager,
            reverse: bool,
            from: Option<usize>,
        ) -> Option<usize> {
            if self.cur_len == 0 {
                return None;
            }

            let solver = self.position_solver(mgr);
            let last_data = self.data.len() - 1;
            let data = if let Some(index) = from {
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

            let (_, action) = self.scroll.focus_rect(solver.rect(data), self.core.rect);
            if !action.is_empty() {
                *mgr |= action;
                self.update_widgets(mgr);
            }

            Some(data % usize::conv(self.cur_len))
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

        fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            let disabled = disabled || self.is_disabled();
            let offset = self.scroll_offset();
            draw.with_clip_region(self.core.rect, offset, &mut |draw| {
                for child in &mut self.widgets[..self.cur_len.cast()] {
                    child.widget.draw(draw, mgr, disabled);
                    if let Some(ref key) = child.key {
                        if self.selection.contains(key) {
                            draw.selection_box(child.widget.rect());
                        }
                    }
                }
            });
        }
    }

    impl SendEvent for Self {
        fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if self.is_disabled() {
                return Response::Unhandled;
            }

            if let Some(index) = self.id().index_of_child(id) {
                let child_event = self.scroll.offset_event(event.clone());
                let response;
                if let Some(child) = self.widgets.get_mut(index) {
                    let r = child.widget.send(mgr, id, child_event);
                    response = (child.key.clone(), r);
                } else {
                    return Response::Unhandled;
                };
                if matches!(&response.1, Response::Update | Response::Msg(_)) {
                    let wd = &self.widgets[index];
                    if let Some(key) = wd.key.as_ref() {
                        if let Some(value) = self.view.get(&wd.widget) {
                            if let Some(handle) = self.data.update(key, value) {
                                mgr.trigger_update(handle, 0);
                            }
                        }
                    }
                }
                match response {
                    (_, Response::None) => return Response::None,
                    (key, Response::Unhandled) => {
                        if let Event::PressStart { source, coord, .. } = event {
                            if source.is_primary() {
                                // We request a grab with our ID, hence the
                                // PressMove/PressEnd events are matched below.
                                if mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None) {
                                    self.press_event = Some(source);
                                    self.press_phase = PressPhase::Start(coord);
                                    self.press_target = key;
                                }
                                return Response::None;
                            }
                        }
                    }
                    (_, Response::Pan(delta)) => {
                        return match self.scroll_by_delta(mgr, delta) {
                            delta if delta == Offset::ZERO => Response::None,
                            delta => Response::Pan(delta),
                        };
                    }
                    (_, Response::Focus(rect)) => {
                        let (rect, action) = self.scroll.focus_rect(rect, self.core.rect);
                        *mgr |= action;
                        self.update_widgets(mgr);
                        return Response::Focus(rect);
                    }
                    (Some(key), Response::Select) => {
                        return match self.sel_mode {
                            SelectionMode::None => Response::None,
                            SelectionMode::Single => {
                                self.selection.clear();
                                self.selection.insert(key.clone());
                                Response::Msg(ChildMsg::Select(key))
                            }
                            SelectionMode::Multiple => {
                                if self.selection.remove(&key) {
                                    Response::Msg(ChildMsg::Deselect(key))
                                } else {
                                    self.selection.insert(key.clone());
                                    Response::Msg(ChildMsg::Select(key))
                                }
                            }
                        };
                    }
                    (None, Response::Select) => return Response::None,
                    (_, Response::Update) => return Response::None,
                    (key, Response::Msg(msg)) => {
                        trace!(
                            "Received by {} from {:?}: {:?}",
                            self.id(),
                            &key,
                            kas::util::TryFormat(&msg)
                        );
                        if let Some(key) = key {
                            if let Some(handle) = self.data.handle(&key, &msg) {
                                mgr.trigger_update(handle, 0);
                            }
                            return Response::Msg(ChildMsg::Child(key, msg));
                        } else {
                            log::warn!("ListView: response from widget with no key");
                            return Response::None;
                        }
                    }
                }
            } else {
                debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
                match event {
                    Event::HandleUpdate { .. } => {
                        // TODO(opt): use the update payload to indicate which widgets need updating?
                        self.update_view(mgr);
                        return Response::Update;
                    }
                    Event::PressMove { source, coord, .. } if self.press_event == Some(source) => {
                        if let PressPhase::Start(start_coord) = self.press_phase {
                            if mgr.config_test_pan_thresh(coord - start_coord) {
                                self.press_phase = PressPhase::Pan;
                            }
                        }
                        match self.press_phase {
                            PressPhase::Pan => {
                                mgr.update_grab_cursor(self.id(), CursorIcon::Grabbing);
                                // fall through to scroll handler
                            }
                            _ => return Response::None,
                        }
                    }
                    Event::PressEnd { source, .. } if self.press_event == Some(source) => {
                        self.press_event = None;
                        if self.press_phase == PressPhase::Pan {
                            return Response::None;
                        }
                        return match self.sel_mode {
                            SelectionMode::None => Response::None,
                            SelectionMode::Single => {
                                self.selection.clear();
                                if let Some(ref key) = self.press_target {
                                    self.selection.insert(key.clone());
                                    ChildMsg::Select(key.clone()).into()
                                } else {
                                    Response::None
                                }
                            }
                            SelectionMode::Multiple => {
                                if let Some(ref key) = self.press_target {
                                    if self.selection.remove(key) {
                                        ChildMsg::Deselect(key.clone()).into()
                                    } else {
                                        self.selection.insert(key.clone());
                                        ChildMsg::Select(key.clone()).into()
                                    }
                                } else {
                                    Response::None
                                }
                            }
                        };
                    }
                    _ => (), // fall through to scroll handler
                }
            };

            let id = self.id();
            if let Event::Command(cmd, _) = event {
                let solver = self.position_solver(mgr);
                let cur = mgr
                    .nav_focus()
                    .and_then(|id| self.find_child_index(id))
                    .map(|index| solver.child_to_data(index));
                let last = self.data.len().wrapping_sub(1);
                let is_vert = self.direction.is_vertical();
                let len = solver.cur_len;

                let data = match (cmd, cur) {
                    _ if last == usize::MAX => None,
                    _ if !self.widgets[0].widget.key_nav() => None,
                    (Command::Home, _) => Some(0),
                    (Command::End, _) => Some(last),
                    (Command::Left, Some(cur)) if !is_vert && cur > 0 => Some(cur - 1),
                    (Command::Up, Some(cur)) if is_vert && cur > 0 => Some(cur - 1),
                    (Command::Right, Some(cur)) if !is_vert && cur < last => Some(cur + 1),
                    (Command::Down, Some(cur)) if is_vert && cur < last => Some(cur + 1),
                    (Command::PageUp, Some(cur)) if cur > 0 => Some(cur.saturating_sub(len / 2)),
                    (Command::PageDown, Some(cur)) if cur < last => Some((cur + len / 2).min(last)),
                    _ => None,
                };
                if let Some(index) = data {
                    // Set nav focus to index and update scroll position
                    let (rect, action) = self.scroll.focus_rect(solver.rect(index), self.core.rect);
                    if !action.is_empty() {
                        *mgr |= action;
                        self.update_widgets(mgr);
                    }
                    let len = usize::conv(self.cur_len);
                    mgr.set_nav_focus(self.widgets[index % len].widget.id(), true);
                    Response::Focus(rect)
                } else {
                    Response::None
                }
            } else {
                let (action, response) =
                    self.scroll
                        .scroll_by_event(event, self.core.rect.size, |source, _, coord| {
                            if source.is_primary() && mgr.config_enable_mouse_pan() {
                                let icon = Some(CursorIcon::Grabbing);
                                mgr.request_grab(id, source, coord, GrabMode::Grab, icon);
                            }
                        });
                if !action.is_empty() {
                    *mgr |= action;
                    self.update_widgets(mgr);
                }
                response.void_into()
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
