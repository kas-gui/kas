// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view widget

use super::{DefaultView, ListData, View};
use kas::event::{CursorIcon, GrabMode, PressSource};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::widget::{ScrollComponent, ScrollWidget};
use linear_map::set::LinearSet;
use log::{debug, trace};
use std::convert::TryFrom;
use std::time::Instant;

/// Selection mode used by [`ListView`]
#[derive(Clone, Copy, Debug, VoidMsg)]
pub enum SelectionMode {
    None,
    Single,
    Multiple,
}
impl Default for SelectionMode {
    fn default() -> Self {
        SelectionMode::None
    }
}

#[derive(Clone, Debug, Default)]
struct WidgetData<K, W> {
    key: Option<K>,
    widget: W,
}

// TODO: do we need to keep the T::Item: Default bound used to initialise entries without data?

/// Message type of [`ListView`]
#[derive(Clone, Debug, VoidMsg)]
pub enum ListMsg<K, M> {
    Select(K),
    Deselect(K),
    Child(K, M),
}

impl<K, M> From<Response<ListMsg<K, M>>> for Response<M> {
    fn from(r: Response<ListMsg<K, M>>) -> Self {
        match Response::try_from(r) {
            Ok(r) => r,
            Err(msg) => match msg {
                ListMsg::Child(_, msg) => Response::Msg(msg),
                _ => Response::None,
            },
        }
    }
}

/// List view widget
#[derive(Clone, Debug, Widget)]
#[handler(send=noauto, msg=ListMsg<T::Key, <V::Widget as Handler>::Msg>)]
#[widget(children=noauto, config=noauto)]
pub struct ListView<D: Directional, T: ListData + 'static, V: View<T::Item> = DefaultView>
where
    T::Item: Default,
{
    first_id: WidgetId,
    #[widget_core]
    core: CoreData,
    offset: Offset,
    frame_size: Size,
    view: V,
    data: T,
    widgets: Vec<WidgetData<T::Key, V::Widget>>,
    cur_len: u32,
    direction: D,
    align_hints: AlignHints,
    ideal_visible: i32,
    child_size_min: i32,
    child_size_ideal: i32,
    child_inter_margin: i32,
    child_skip: i32,
    child_size: Size,
    scroll: ScrollComponent,
    sel_mode: SelectionMode,
    // TODO(opt): replace selection list with RangeOrSet type?
    selection: LinearSet<T::Key>,
    press_event: Option<PressSource>,
    press_target: Option<T::Key>,
}

impl<D: Directional + Default, T: ListData, V: View<T::Item> + Default> ListView<D, T, V>
where
    T::Item: Default,
{
    /// Construct a new instance
    ///
    /// This constructor is available where the direction is determined by the
    /// type: for `D: Directional + Default`. In other cases, use
    /// [`ListView::new_with_direction`].
    pub fn new(data: T) -> Self {
        Self::new_with_dir_view(D::default(), <V as Default>::default(), data)
    }
}
impl<D: Directional, T: ListData, V: View<T::Item> + Default> ListView<D, T, V>
where
    T::Item: Default,
{
    /// Construct a new instance with explicit direction
    pub fn new_with_direction(direction: D, data: T) -> Self {
        Self::new_with_dir_view(direction, <V as Default>::default(), data)
    }
}
impl<D: Directional + Default, T: ListData, V: View<T::Item> + Default> ListView<D, T, V>
where
    T::Item: Default,
{
    /// Construct a new instance with explicit view
    pub fn new_with_view(view: V, data: T) -> Self {
        Self::new_with_dir_view(D::default(), view, data)
    }
}
impl<D: Directional, T: ListData, V: View<T::Item>> ListView<D, T, V>
where
    T::Item: Default,
{
    /// Construct a new instance with explicit direction and view
    pub fn new_with_dir_view(direction: D, view: V, data: T) -> Self {
        ListView {
            first_id: Default::default(),
            core: Default::default(),
            offset: Default::default(),
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
            child_skip: 0,
            child_size: Size::ZERO,
            scroll: Default::default(),
            sel_mode: SelectionMode::None,
            selection: Default::default(),
            press_event: None,
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
    pub fn selected_iter<'a>(&'a self) -> impl Iterator<Item = &'a T::Key> + 'a {
        self.selection.iter()
    }

    /// Check whether an entry is selected
    pub fn is_selected(&self, key: &T::Key) -> bool {
        self.selection.contains(key)
    }

    /// Clear all selected items
    ///
    /// Does not send [`ListMsg`] responses.
    pub fn clear_selected(&mut self) {
        self.selection.clear();
    }

    /// Directly select an item
    ///
    /// Returns `true` if selected, `false` if already selected.
    ///
    /// Does not send [`ListMsg`] responses.
    pub fn select(&mut self, key: T::Key) -> bool {
        self.selection.insert(key)
    }

    /// Directly deselect an item
    ///
    /// Returns `true` if deselected, `false` if not previously selected.
    ///
    /// Does not send [`ListMsg`] responses.
    pub fn deselect(&mut self, key: &T::Key) -> bool {
        self.selection.remove(key)
    }

    /// Manually trigger an update to handle changed data
    pub fn update_view(&mut self, mgr: &mut Manager) {
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

    fn update_widgets(&mut self, mgr: &mut Manager) {
        let time = Instant::now();
        // set_rect allocates enough widgets to view a page; we update widget-data allocations
        let len = self.widgets.len().min(self.data.len());
        self.cur_len = len.cast();
        let offset = u64::conv(self.scroll_offset().extract(self.direction));
        let first_data = usize::conv(offset / u64::conv(self.child_skip));
        let mut skip = match self.direction.is_vertical() {
            false => Offset(self.child_skip, 0),
            true => Offset(0, self.child_skip),
        };
        let mut pos_start = self.core.rect.pos + self.offset;
        if self.direction.is_reversed() {
            pos_start += skip * i32::conv(len - 1);
            skip = skip * -1;
        }
        let mut rect = Rect::new(pos_start, self.child_size);
        let mut action = TkAction::empty();
        for (i, item) in self
            .data
            .iter_vec_from(first_data, len)
            .into_iter()
            .enumerate()
        {
            let i = first_data + i;
            let key = Some(item.0);
            let w = &mut self.widgets[i % len];
            if key != w.key {
                w.key = key;
                action |= self.view.set(&mut w.widget, item.1);
            }
            // TODO(opt): don't need to set_rect on all widgets when scrolling
            rect.pos = pos_start + skip * i32::conv(i);
            w.widget.set_rect(mgr, rect, self.align_hints);
        }
        *mgr |= action;
        let dur = (Instant::now() - time).as_micros();
        trace!("ListView::update_widgets completed in {}μs", dur);
    }
}

impl<D: Directional, T: ListData, V: View<T::Item>> ScrollWidget for ListView<D, T, V>
where
    T::Item: Default,
{
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

impl<D: Directional, T: ListData, V: View<T::Item>> WidgetChildren for ListView<D, T, V>
where
    T::Item: Default,
{
    #[inline]
    fn first_id(&self) -> WidgetId {
        self.first_id
    }
    fn record_first_id(&mut self, id: WidgetId) {
        self.first_id = id;
    }
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

impl<D: Directional, T: ListData, V: View<T::Item>> WidgetConfig for ListView<D, T, V>
where
    T::Item: Default,
{
    fn configure(&mut self, mgr: &mut Manager) {
        if let Some(handle) = self.data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
        mgr.register_nav_fallback(self.id());
    }
}

impl<D: Directional, T: ListData, V: View<T::Item>> Layout for ListView<D, T, V>
where
    T::Item: Default,
{
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        // We use an invisible frame for highlighting selections, drawing into the margin
        let inner_margin = size_handle.inner_margin().extract(axis);
        let frame = FrameRules::new_sym(0, inner_margin, (0, 0));

        // We initialise the first widget if possible, otherwise use V::Widget::default()
        if self.widgets.is_empty() {
            let (key, widget) = if let Some((key, data)) = self.data.iter_vec(1).into_iter().next()
            {
                (Some(key), self.view.new(data))
            } else {
                (None, self.view.default())
            };
            self.widgets.push(WidgetData { key, widget });
        }
        let mut rules = self.widgets[0].widget.size_rules(size_handle, axis);
        if axis.is_vertical() == self.direction.is_vertical() {
            self.child_size_min = rules.min_size();
            self.child_size_ideal = rules.ideal_size();
            let m = rules.margins_i32();
            self.child_inter_margin = (m.0 + m.1).max(inner_margin);
            rules.multiply_with_margin(2, self.ideal_visible);
            rules.set_stretch(rules.stretch().max(StretchPolicy::HighUtility));
        }
        let (rules, offset, size) = frame.surround(rules);
        self.offset.set_component(axis, offset);
        self.frame_size.set_component(axis, size);
        rules
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, mut align: AlignHints) {
        self.core.rect = rect;

        let data_len = self.data.len();
        let data_len32 = i32::try_from(data_len).unwrap();
        let mut child_size = rect.size - self.frame_size;
        let mut content_size = rect.size;
        let skip;
        let num;
        if self.direction.is_horizontal() {
            if child_size.0 >= self.ideal_visible * self.child_size_ideal {
                child_size.0 = self.child_size_ideal;
            } else {
                child_size.0 = self.child_size_min;
            }
            skip = Offset(child_size.0 + self.child_inter_margin, 0);
            self.child_skip = skip.0;
            align.horiz = None;
            num = (rect.size.0 + skip.0 - 1) / skip.0 + 1;

            content_size.0 = (skip.0 * data_len32 - self.child_inter_margin).max(0);
        } else {
            if child_size.1 >= self.ideal_visible * self.child_size_ideal {
                child_size.1 = self.child_size_ideal;
            } else {
                child_size.1 = self.child_size_min;
            }
            skip = Offset(0, child_size.1 + self.child_inter_margin);
            self.child_skip = skip.1;
            align.vert = None;
            num = (rect.size.1 + skip.1 - 1) / skip.1 + 1;

            content_size.1 = (skip.1 * data_len32 - self.child_inter_margin).max(0);
        }

        self.child_size = child_size;
        self.align_hints = align;

        let old_num = self.widgets.len();
        let num = usize::conv(num);
        if old_num < num {
            debug!("allocating widgets (old len = {}, new = {})", old_num, num);
            *mgr |= TkAction::RECONFIGURE;
            self.widgets.reserve(num - old_num);
            mgr.size_handle(|size_handle| {
                for (key, item) in self.data.iter_vec_from(old_num, num - old_num) {
                    let key = Some(key);
                    let mut widget = self.view.new(item);
                    // We must solve size rules on new widgets:
                    solve_size_rules(
                        &mut widget,
                        size_handle,
                        Some(child_size.0),
                        Some(child_size.1),
                    );
                    self.widgets.push(WidgetData { key, widget });
                }
                for _ in self.widgets.len()..num {
                    let mut widget = self.view.default();
                    solve_size_rules(
                        &mut widget,
                        size_handle,
                        Some(child_size.0),
                        Some(child_size.1),
                    );
                    self.widgets.push(WidgetData { key: None, widget });
                }
            });
        } else if num + 64 <= self.widgets.len() {
            // Free memory (rarely useful?)
            self.widgets.truncate(num);
        }
        *mgr |= self.scroll.set_sizes(rect.size, content_size);
        self.update_widgets(mgr);
    }

    fn spatial_range(&self) -> (usize, usize) {
        let last = self.num_children().wrapping_sub(1);
        match self.direction.is_reversed() {
            false => (0, last),
            true => (last, 0),
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }

        let coord = coord + self.scroll.offset();
        for child in &self.widgets[..self.cur_len.cast()] {
            if let Some(id) = child.widget.find_id(coord) {
                return Some(id);
            }
        }
        Some(self.id())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        let offset = self.scroll_offset();
        use kas::draw::ClipRegion::Scroll;
        draw_handle.clip_region(self.core.rect, offset, Scroll, &mut |draw_handle| {
            for child in &self.widgets[..self.cur_len.cast()] {
                child.widget.draw(draw_handle, mgr, disabled);
                if let Some(ref key) = child.key {
                    if self.is_selected(key) {
                        draw_handle.selection_box(child.widget.rect());
                    }
                }
            }
        });
    }
}

impl<D: Directional, T: ListData, V: View<T::Item>> SendEvent for ListView<D, T, V>
where
    T::Item: Default,
{
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        let event = if id < self.id() {
            let response = 'outer: loop {
                for child in &mut self.widgets[..self.cur_len.cast()] {
                    if id <= child.widget.id() {
                        let event = self.scroll.offset_event(event);
                        break 'outer (child.key.clone(), child.widget.send(mgr, id, event));
                    }
                }
                debug_assert!(false, "SendEvent::send: bad WidgetId");
                return Response::Unhandled(event);
            };
            match response {
                (key, Response::Unhandled(event)) => {
                    if let Event::PressStart { source, coord, .. } = event {
                        if source.is_primary() {
                            // We request a grab with our ID, hence the
                            // PressMove/PressEnd events are matched below.
                            if mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None) {
                                self.press_event = Some(source);
                                self.press_target = key;
                            }
                            return Response::None;
                        }
                    }
                    event
                }
                (_, Response::Focus(rect)) => {
                    let (rect, action) = self.scroll.focus_rect(rect, self.core.rect);
                    *mgr |= action;
                    self.update_widgets(mgr);
                    return Response::Focus(rect);
                }
                (key, r) => {
                    return match Response::try_from(r) {
                        Ok(r) => r,
                        Err(msg) => {
                            if let Some(key) = key {
                                Response::Msg(ListMsg::Child(key, msg))
                            } else {
                                log::warn!("ListView: response from widget with no key");
                                Response::None
                            }
                        }
                    }
                }
            }
        } else {
            debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
            match event {
                Event::HandleUpdate { .. } => {
                    self.update_view(mgr);
                    return Response::None;
                }
                Event::PressMove { source, .. } if self.press_event == Some(source) => {
                    self.press_event = None;
                    mgr.update_grab_cursor(self.id(), CursorIcon::Grabbing);
                    event // fall through to scroll handler
                }
                Event::PressEnd { source, .. } if self.press_event == Some(source) => {
                    self.press_event = None;
                    return match self.sel_mode {
                        SelectionMode::None => Response::None,
                        SelectionMode::Single => {
                            self.selection.clear();
                            if let Some(ref key) = self.press_target {
                                self.selection.insert(key.clone());
                                ListMsg::Select(key.clone()).into()
                            } else {
                                Response::None
                            }
                        }
                        SelectionMode::Multiple => {
                            if let Some(ref key) = self.press_target {
                                if self.selection.remove(key) {
                                    ListMsg::Deselect(key.clone()).into()
                                } else {
                                    self.selection.insert(key.clone());
                                    ListMsg::Select(key.clone()).into()
                                }
                            } else {
                                Response::None
                            }
                        }
                    };
                }
                event => event,
            }
        };

        let id = self.id();
        let (action, response) =
            self.scroll
                .scroll_by_event(event, self.core.rect.size, |source, _, coord| {
                    if source.is_primary() {
                        let icon = Some(CursorIcon::Grabbing);
                        mgr.request_grab(id, source, coord, GrabMode::Grab, icon);
                    }
                });
        if !action.is_empty() {
            *mgr |= action;
            self.update_widgets(mgr);
            Response::Focus(self.rect())
        } else {
            response.void_into()
        }
    }
}
