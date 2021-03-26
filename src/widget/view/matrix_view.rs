// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view widget

use super::{driver, Driver, MatrixData, SelectionMode};
use kas::event::{ChildMsg, CursorIcon, GrabMode, PressSource};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::updatable::UpdatableAll;
#[allow(unused)] // doc links
use kas::widget::ScrollBars;
use kas::widget::{ScrollComponent, Scrollable};
use linear_map::set::LinearSet;
use log::{debug, trace};
use std::time::Instant;

#[derive(Clone, Debug, Default)]
struct WidgetData<K, W> {
    key: Option<K>,
    widget: W,
}

/// Matrix view widget
///
/// This widget supports a view over a matrix of shared data items.
///
/// The shared data type `T` must support [`MatrixData`] and
/// [`UpdatableAll`], the latter with key type `T::Key` and message type
/// matching the widget's message. One may use [`kas::widget::view::SharedRc`]
/// or a custom shared data type.
///
/// The driver `V` must implement [`Driver`], with data type
/// `<T as MatrixData>::Item`. Several implementations are available in the
/// [`driver`] module or a custom implementation may be used.
///
/// This widget is [`Scrollable`], supporting keyboard, wheel and drag
/// scrolling. You may wish to wrap this widget with [`ScrollBars`].
#[derive(Clone, Debug, Widget)]
#[handler(send=noauto, msg=ChildMsg<T::Key, <V::Widget as Handler>::Msg>)]
#[widget(children=noauto, config=noauto)]
pub struct MatrixView<
    T: MatrixData + UpdatableAll<T::Key, V::Msg> + 'static,
    V: Driver<T::Item> = driver::Default,
> {
    first_id: WidgetId,
    #[widget_core]
    core: CoreData,
    offset: Offset,
    frame_size: Size,
    view: V,
    data: T,
    widgets: Vec<WidgetData<T::Key, V::Widget>>,
    // TODO: the following three all have units of "the number of rows/cols"
    ideal_len: Size,
    alloc_len: Size,
    cur_len: Size,
    child_size_min: Size,
    child_size_ideal: Size,
    child_inter_margin: Size,
    child_size: Size,
    scroll: ScrollComponent,
    sel_mode: SelectionMode,
    // TODO(opt): replace selection list with RangeOrSet type?
    selection: LinearSet<T::Key>,
    press_event: Option<PressSource>,
    press_target: Option<T::Key>,
}

impl<T: MatrixData + UpdatableAll<T::Key, V::Msg>, V: Driver<T::Item> + Default> MatrixView<T, V> {
    /// Construct a new instance
    pub fn new(data: T) -> Self {
        Self::new_with_driver(<V as Default>::default(), data)
    }
}
impl<T: MatrixData + UpdatableAll<T::Key, V::Msg>, V: Driver<T::Item>> MatrixView<T, V> {
    /// Construct a new instance with explicit view
    pub fn new_with_driver(view: V, data: T) -> Self {
        MatrixView {
            first_id: Default::default(),
            core: Default::default(),
            offset: Default::default(),
            frame_size: Default::default(),
            view,
            data,
            widgets: Default::default(),
            ideal_len: Size(5, 3),
            alloc_len: Size::ZERO,
            cur_len: Size::ZERO,
            child_size_min: Size::ZERO,
            child_size_ideal: Size::ZERO,
            child_inter_margin: Size::ZERO,
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
    /// It may be necessary to use [`MatrixView::update_view`] to update the view of this data.
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
    /// [`MatrixData::update`]). Other widgets sharing this data are notified
    /// of the update, if data is changed.
    pub fn set_value(&self, mgr: &mut Manager, key: &T::Key, data: T::Item) {
        if let Some(handle) = self.data.update(key, data) {
            mgr.trigger_update(handle, 0);
        }
    }

    /// Update shared data
    ///
    /// This is purely a convenience method over [`MatrixView::set_value`].
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
    pub fn selected_iter<'a>(&'a self) -> impl Iterator<Item = &'a T::Key> + 'a {
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
    pub fn select(&mut self, key: T::Key) -> Result<bool, ()> {
        match self.sel_mode {
            SelectionMode::None => return Err(()),
            SelectionMode::Single => self.selection.clear(),
            _ => (),
        }
        if !self.data.contains(&key) {
            return Err(());
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
        self.selection.retain(|key| data.contains(key));
        for w in &mut self.widgets {
            w.key = None;
        }
        self.update_widgets(mgr);
        // Force SET_SIZE so that scroll-bar wrappers get updated
        trace!("update_view triggers SET_SIZE");
        *mgr |= TkAction::SET_SIZE;
    }

    /// Set the preferred number of items visible (inline)
    ///
    /// This affects the (ideal) size request and whether children are sized
    /// according to their ideal or minimum size but not the minimum size.
    pub fn with_num_visible(mut self, cols: i32, rows: i32) -> Self {
        self.ideal_len = Size(cols, rows);
        self
    }

    fn update_widgets(&mut self, mgr: &mut Manager) {
        let time = Instant::now();

        let data_len = Size(self.data.col_len().cast(), self.data.row_len().cast());
        let view_size = self.rect().size;
        let skip = self.child_size + self.child_inter_margin;
        let content_size = (skip.cwise_mul(data_len) - self.child_inter_margin).max(Size::ZERO);
        *mgr |= self.scroll.set_sizes(view_size, content_size);

        let offset = self.scroll_offset();
        let first_col = usize::conv(u64::conv(offset.0) / u64::conv(skip.0));
        let first_row = usize::conv(u64::conv(offset.1) / u64::conv(skip.1));
        let cols = self
            .data
            .col_iter_vec_from(first_col, self.alloc_len.0.cast());
        let rows = self
            .data
            .row_iter_vec_from(first_row, self.alloc_len.1.cast());
        self.cur_len = Size(cols.len().cast(), rows.len().cast());

        let pos_start = self.core.rect.pos + self.offset;
        let mut rect = Rect::new(pos_start, self.child_size);

        let mut action = TkAction::empty();
        for (cn, col) in cols.iter().enumerate() {
            let ci = first_col + cn;
            for (rn, row) in rows.iter().enumerate() {
                let ri = first_row + rn;
                let i = (ci % cols.len()) * rows.len() + (ri % rows.len());
                let w = &mut self.widgets[i];
                let key = T::make_key(&col, &row);
                if w.key.as_ref() != Some(&key) {
                    if let Some(item) = self.data.get_cloned(&key) {
                        w.key = Some(key.clone());
                        action |= self.view.set(&mut w.widget, item);
                    } else {
                        w.key = None; // disables drawing and clicking
                    }
                }
                rect.pos = pos_start + skip.cwise_mul(Size(ci.cast(), ri.cast()));
                if w.widget.rect() != rect {
                    w.widget.set_rect(mgr, rect, Default::default());
                }
            }
        }
        *mgr |= action;
        let dur = (Instant::now() - time).as_micros();
        trace!("MatrixView::update_widgets completed in {}μs", dur);
    }
}

impl<T: MatrixData + UpdatableAll<T::Key, V::Msg>, V: Driver<T::Item>> Scrollable
    for MatrixView<T, V>
{
    fn scroll_axes(&self, size: Size) -> (bool, bool) {
        let item_min = self.child_size_min + self.child_inter_margin;
        let data_len = Size(self.data.col_len().cast(), self.data.row_len().cast());
        let min_size = (item_min.cwise_mul(data_len) - self.child_inter_margin).max(Size::ZERO);
        (min_size.0 > size.0, min_size.1 > size.1)
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

impl<T: MatrixData + UpdatableAll<T::Key, V::Msg>, V: Driver<T::Item>> WidgetChildren
    for MatrixView<T, V>
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

impl<T: MatrixData + UpdatableAll<T::Key, V::Msg>, V: Driver<T::Item>> WidgetConfig
    for MatrixView<T, V>
{
    fn configure(&mut self, mgr: &mut Manager) {
        self.data.enable_recursive_updates(mgr);
        if let Some(handle) = self.data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
        mgr.register_nav_fallback(self.id());
    }
}

impl<T: MatrixData + UpdatableAll<T::Key, V::Msg>, V: Driver<T::Item>> Layout for MatrixView<T, V> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        // We use an invisible frame for highlighting selections, drawing into the margin
        let inner_margin = size_handle.inner_margin().extract(axis);
        let frame = FrameRules::new_sym(0, inner_margin, 0);

        // We use a default-generated widget to generate size rules
        let mut rules = self.view.new().size_rules(size_handle, axis);

        self.child_size_min.set_component(axis, rules.min_size());
        self.child_size_ideal
            .set_component(axis, rules.ideal_size());
        let m = rules.margins_i32();
        self.child_inter_margin
            .set_component(axis, (m.0 + m.1).max(inner_margin));

        rules.multiply_with_margin(2, self.ideal_len.extract(axis));
        rules.set_stretch(rules.stretch().max(Stretch::High));

        let (rules, offset, size) = frame.surround(rules);
        self.offset.set_component(axis, offset);
        self.frame_size.set_component(axis, size);
        rules
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, _align: AlignHints) {
        self.core.rect = rect;

        let mut child_size = rect.size - self.frame_size;
        if child_size.0 >= self.ideal_len.0 * self.child_size_ideal.0 {
            child_size.0 = self.child_size_ideal.0;
        } else {
            child_size.0 = self.child_size_min.0;
        }
        if child_size.1 >= self.ideal_len.1 * self.child_size_ideal.1 {
            child_size.1 = self.child_size_ideal.1;
        } else {
            child_size.1 = self.child_size_min.1;
        }
        self.child_size = child_size;

        let skip = child_size + self.child_inter_margin;
        let vis_len = (rect.size + skip - Size::splat(1)).cwise_div(skip) + Size::splat(1);
        self.alloc_len = vis_len;

        let old_num = self.widgets.len();
        let num = usize::conv(vis_len.0) * usize::conv(vis_len.1);
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
        } else if num + 64 <= self.widgets.len() {
            // Free memory (rarely useful?)
            self.widgets.truncate(num);
        }
        self.update_widgets(mgr);
    }

    fn spatial_nav(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        let cur_len = usize::conv(self.cur_len.0) * usize::conv(self.cur_len.1);
        if cur_len == 0 {
            return None;
        }

        // TODO: if last row/col is completely hidden, this and cur_len should be less
        let last = cur_len - 1;

        if let Some(index) = from {
            let p = self.widgets[index].widget.rect().pos;
            let index = match reverse {
                false if index < last => index + 1,
                false => 0,
                true if 0 < index => index - 1,
                true => last,
            };
            let q = self.widgets[index].widget.rect().pos;
            match reverse {
                false if q.1 > p.1 || (q.1 == p.1 && q.0 > p.0) => Some(index),
                true if q.1 < p.1 || (q.1 == p.1 && q.0 < p.0) => Some(index),
                _ => None,
            }
        } else {
            // Simplified version of logic in update_widgets
            let skip = self.child_size + self.child_inter_margin;
            let offset = self.scroll_offset();
            let ci = usize::conv(u64::conv(offset.0) / u64::conv(skip.0));
            let ri = usize::conv(u64::conv(offset.1) / u64::conv(skip.1));
            let (cols, rows): (usize, usize) = (self.cur_len.0.cast(), self.cur_len.1.cast());
            let mut data = (ci % cols) * rows + (ri % rows);
            if reverse {
                data += last;
            }
            Some(data % cur_len)
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }

        let coord = coord + self.scroll.offset();
        let num = usize::conv(self.cur_len.0) * usize::conv(self.cur_len.1);
        for child in &self.widgets[..num] {
            if child.key.is_some() {
                if let Some(id) = child.widget.find_id(coord) {
                    return Some(id);
                }
            }
        }
        Some(self.id())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        let offset = self.scroll_offset();
        use kas::draw::ClipRegion::Scroll;
        let num = usize::conv(self.cur_len.0) * usize::conv(self.cur_len.1);
        draw_handle.clip_region(self.core.rect, offset, Scroll, &mut |draw_handle| {
            for child in &self.widgets[..num] {
                if let Some(ref key) = child.key {
                    child.widget.draw(draw_handle, mgr, disabled);
                    if self.is_selected(key) {
                        draw_handle.selection_box(child.widget.rect());
                    }
                }
            }
        });
    }
}

impl<T: MatrixData + UpdatableAll<T::Key, V::Msg>, V: Driver<T::Item>> SendEvent
    for MatrixView<T, V>
{
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled;
        }

        if id < self.id() {
            let child_event = self.scroll.offset_event(event.clone());
            let response = 'outer: loop {
                // We forward events to all children, even if not visible
                // (e.g. these may be subscribed to an UpdateHandle).
                for child in self.widgets.iter_mut() {
                    if id <= child.widget.id() {
                        let r = child.widget.send(mgr, id, child_event);
                        break 'outer (child.key.clone(), r);
                    }
                }
                debug_assert!(false, "SendEvent::send: bad WidgetId");
                return Response::Unhandled;
            };
            match response {
                (_, Response::None) => return Response::None,
                (key, Response::Unhandled) => {
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
                }
                (_, Response::Focus(rect)) => {
                    let (rect, action) = self.scroll.focus_rect(rect, self.core.rect);
                    *mgr |= action;
                    self.update_widgets(mgr);
                    return Response::Focus(rect);
                }
                (Some(key), Response::Select) => {
                    match self.sel_mode {
                        SelectionMode::None => (),
                        SelectionMode::Single => {
                            self.selection.clear();
                            self.selection.insert(key);
                        }
                        SelectionMode::Multiple => {
                            if !self.selection.remove(&key) {
                                self.selection.insert(key);
                            }
                        }
                    }
                    return Response::None;
                }
                (None, Response::Select) => return Response::None,
                (_, Response::Update) => return Response::None,
                (key, Response::Msg(msg)) => {
                    if let Some(key) = key {
                        if let Some(handle) = self.data.handle(&key, &msg) {
                            mgr.trigger_update(handle, 0);
                        }
                        return Response::Msg(ChildMsg::Child(key, msg));
                    } else {
                        log::warn!("MatrixView: response from widget with no key");
                        return Response::None;
                    }
                }
            }
        } else {
            debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
            match event {
                Event::HandleUpdate { .. } => {
                    self.update_view(mgr);
                    return Response::Update;
                }
                Event::PressMove { source, .. } if self.press_event == Some(source) => {
                    self.press_event = None;
                    mgr.update_grab_cursor(self.id(), CursorIcon::Grabbing);
                    // fall through to scroll handler
                }
                Event::PressEnd { source, .. } if self.press_event == Some(source) => {
                    self.press_event = None;
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
            Response::Focus(self.rect())
        } else {
            response.void_into()
        }
    }
}
