// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view widget

use super::{Accessor, DefaultView, ViewWidget};
use kas::event::{CursorIcon, GrabMode, PressSource};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::text::Range;
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

// TODO: do we need to keep the A::Item: Default bound used by allocate?

/// List view widget
#[derive(Clone, Debug, Widget)]
#[handler(send=noauto, msg=<W as Handler>::Msg)]
#[widget(children=noauto, config=noauto)]
pub struct ListView<
    D: Directional,
    A: Accessor<usize>,
    W: ViewWidget<A::Item> = <<A as Accessor<usize>>::Item as DefaultView>::Widget,
> where
    A::Item: Default,
{
    first_id: WidgetId,
    #[widget_core]
    core: CoreData,
    offset: Offset,
    frame_size: Size,
    data: A,
    widgets: Vec<W>,
    direction: D,
    data_range: Range,
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
    selection: LinearSet<u32>,
    press_event: Option<PressSource>,
    press_target: u32,
}

impl<D: Directional + Default, A: Accessor<usize>, W: ViewWidget<A::Item>> ListView<D, A, W>
where
    A::Item: Default,
{
    /// Construct a new instance
    ///
    /// This constructor is available where the direction is determined by the
    /// type: for `D: Directional + Default`. In other cases, use
    /// [`ListView::new_with_direction`].
    pub fn new(data: A) -> Self {
        ListView {
            first_id: Default::default(),
            core: Default::default(),
            offset: Default::default(),
            frame_size: Default::default(),
            data,
            widgets: Default::default(),
            direction: Default::default(),
            data_range: Range::from(0usize..0),
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
            press_target: 0,
        }
    }
}
impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> ListView<D, A, W>
where
    A::Item: Default,
{
    /// Construct a new instance with explicit direction
    pub fn new_with_direction(direction: D, data: A) -> Self {
        ListView {
            first_id: Default::default(),
            core: Default::default(),
            offset: Default::default(),
            frame_size: Default::default(),
            data,
            widgets: Default::default(),
            direction,
            data_range: Range::from(0usize..0),
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
            press_target: 0,
        }
    }

    /// Access the stored data
    pub fn data(&self) -> &A {
        &self.data
    }

    /// Mutably access the stored data
    ///
    /// It may be necessary to use [`ListView::update_view`] to update the view of this data.
    pub fn data_mut(&mut self) -> &mut A {
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
    pub fn selected_iter<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        self.selection.iter().map(|v| (*v).cast())
    }

    /// Check whether an entry is selected
    pub fn is_selected(&self, index: usize) -> bool {
        self.selection.contains(&index.cast())
    }

    /// Manually trigger an update to handle changed data
    pub fn update_view(&mut self, mgr: &mut Manager) {
        self.data_range.end = self.data_range.start;
        self.update_widgets(mgr, true);
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

    // all: whether to update all widgets; if not, assume only scrolling occurred
    fn update_widgets(&mut self, mgr: &mut Manager, all: bool) {
        let time = Instant::now();
        // set_rect allocates enough widgets to view a page; we update widget-data allocations
        // TODO: we may wish to notify self.data of the range it should cache
        let w_len = self.widgets.len();
        let (old_start, old_end) = (self.data_range.start(), self.data_range.end());
        let offset = u64::conv(self.scroll_offset().extract(self.direction));
        let mut first_data = usize::conv(offset / u64::conv(self.child_skip));
        first_data = (first_data + w_len)
            .min(self.data.len())
            .saturating_sub(w_len);
        let data_range = first_data..(first_data + w_len).min(self.data.len());
        let mut skip = match self.direction.is_vertical() {
            false => Offset(self.child_skip, 0),
            true => Offset(0, self.child_skip),
        };
        let mut pos_start = self.core.rect.pos + self.offset;
        if self.direction.is_reversed() {
            pos_start += skip * i32::conv(w_len - 1);
            skip = skip * -1;
        }
        let mut rect = Rect::new(pos_start, self.child_size);
        let mut action = TkAction::empty();
        for data_num in data_range.clone() {
            if all || (data_num < old_start || data_num >= old_end) {
                let w = &mut self.widgets[data_num % w_len];
                action |= w.set(self.data.get(data_num));
                action |= TkAction::REGION_MOVED; // widget moved
                rect.pos = pos_start + skip * i32::conv(data_num);
                w.set_rect(mgr, rect, self.align_hints);
            }
        }
        *mgr |= action;
        self.data_range = data_range.into();
        let dur = (Instant::now() - time).as_micros();
        trace!("ListView::update_widgets completed in {}μs", dur);
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> ScrollWidget for ListView<D, A, W>
where
    A::Item: Default,
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
        self.update_widgets(mgr, false);
        self.scroll.offset()
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> WidgetChildren
    for ListView<D, A, W>
where
    A::Item: Default,
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
        self.widgets.get(index).map(|w| w.as_widget())
    }
    #[inline]
    fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
        self.widgets.get_mut(index).map(|w| w.as_widget_mut())
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> WidgetConfig for ListView<D, A, W>
where
    A::Item: Default,
{
    fn configure(&mut self, mgr: &mut Manager) {
        if let Some(handle) = self.data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
        mgr.register_nav_fallback(self.id());
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> Layout for ListView<D, A, W>
where
    A::Item: Default,
{
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        // We use an invisible frame for highlighting selections, drawing into the margin
        let inner_margin = size_handle.inner_margin().extract(axis);
        let frame = FrameRules::new_sym(0, inner_margin, (0, 0));

        if self.widgets.is_empty() {
            if self.data.len() > 0 {
                self.widgets.push(W::new(self.data.get(0)));
            } else {
                self.widgets.push(W::default());
            }
        }
        let mut rules = self.widgets[0].size_rules(size_handle, axis);
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
        let num = (usize::conv(num)).min(data_len);
        if num > old_num {
            debug!("allocating widgets (old len = {}, new = {})", old_num, num);
            *mgr |= TkAction::RECONFIGURE;
            self.widgets.reserve(num);
            mgr.size_handle(|size_handle| {
                for i in old_num..num {
                    let mut w = W::new(self.data.get(i));
                    // We must solve size rules on new widgets:
                    solve_size_rules(&mut w, size_handle, Some(child_size.0), Some(child_size.1));
                    self.widgets.push(w);
                }
            });
        } else if num + 64 <= old_num {
            // Free memory (rarely useful?)
            self.widgets.truncate(num);
        }
        *mgr |= self.scroll.set_sizes(rect.size, content_size);
        self.update_widgets(mgr, true);
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
        for child in &self.widgets[..self.data_range.len()] {
            if let Some(id) = child.find_id(coord) {
                return Some(id);
            }
        }
        Some(self.id())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        let offset = self.scroll_offset();
        use kas::draw::ClipRegion::Scroll;
        let w_len = self.widgets.len();
        let start = (self.data_range.start() / w_len) * w_len;
        draw_handle.clip_region(self.core.rect, offset, Scroll, &mut |draw_handle| {
            for (i, child) in self.widgets[..self.data_range.len()].iter().enumerate() {
                child.draw(draw_handle, mgr, disabled);
                let mut d = start + i;
                if d < self.data_range.start() {
                    d += w_len;
                }
                if self.is_selected(d) {
                    draw_handle.selection_box(child.rect());
                }
            }
        });
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> SendEvent for ListView<D, A, W>
where
    A::Item: Default,
{
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        let event = if id < self.id() {
            let response = 'outer: loop {
                for (i, child) in self.widgets[..self.data_range.len()].iter_mut().enumerate() {
                    if id <= child.id() {
                        let event = self.scroll.offset_event(event);
                        break 'outer (i, child.send(mgr, id, event));
                    }
                }
                debug_assert!(false, "SendEvent::send: bad WidgetId");
                return Response::Unhandled(event);
            };
            match response {
                (i, Response::Unhandled(event)) => {
                    if let Event::PressStart { source, coord, .. } = event {
                        if source.is_primary() {
                            // We request a grab with our ID, hence the
                            // PressMove/PressEnd events are matched below.
                            if mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None) {
                                self.press_event = Some(source);
                                let w_len = self.widgets.len();
                                let start = (self.data_range.start() / w_len) * w_len;
                                let mut d = start + i;
                                if d < self.data_range.start() {
                                    d += w_len;
                                }
                                self.press_target = d.cast();
                            }
                            return Response::None;
                        }
                    }
                    event
                }
                (_, Response::Focus(rect)) => {
                    let (rect, action) = self.scroll.focus_rect(rect, self.core.rect);
                    *mgr |= action;
                    self.update_widgets(mgr, false);
                    return Response::Focus(rect);
                }
                (_, r) => return r,
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
                    match self.sel_mode {
                        SelectionMode::None => (),
                        SelectionMode::Single => {
                            self.selection.clear();
                            self.selection.insert(self.press_target);
                        }
                        SelectionMode::Multiple => {
                            if !self.selection.remove(&self.press_target) {
                                self.selection.insert(self.press_target);
                            }
                        }
                    }
                    return Response::None;
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
            self.update_widgets(mgr, false);
            Response::Focus(self.rect())
        } else {
            response.void_into()
        }
    }
}
