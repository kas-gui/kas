// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view widget

use super::{Accessor, DefaultView, ViewWidget};
use kas::draw::ClipRegion;
use kas::event::{CursorIcon, GrabMode};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::widget::ScrollComponent;
use std::convert::TryFrom;

// TODO: do we need to keep the A::Item: Default bound used by allocate?

/// List view widget
#[handler(send=noauto, msg=<W as Handler>::Msg)]
#[widget(children=noauto)]
#[derive(Clone, Default, Debug, Widget)]
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
    data: A,
    widgets: Vec<W>,
    direction: D,
    ideal_visible: u32,
    child_size_min: u32,
    child_size_ideal: u32,
    child_inter_margin: u32,
    scroll: ScrollComponent,
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
            data,
            widgets: Default::default(),
            direction: Default::default(),
            ideal_visible: 5,
            child_size_min: 0,
            child_size_ideal: 0,
            child_inter_margin: 0,
            scroll: Default::default(),
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
            data,
            widgets: Default::default(),
            direction,
            ideal_visible: 5,
            child_size_min: 0,
            child_size_ideal: 0,
            child_inter_margin: 0,
            scroll: Default::default(),
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
    pub fn with_num_visible(mut self, number: u32) -> Self {
        self.ideal_visible = number;
        self
    }

    /// Get the maximum scroll offset
    ///
    /// Note: the minimum scroll offset is always zero.
    #[inline]
    pub fn max_scroll_offset(&self) -> Coord {
        self.scroll.max_offset()
    }

    /// Get the current scroll offset
    ///
    /// Contents of the scroll region are translated by this offset (to convert
    /// coordinates from the outer region to the scroll region, add this offset).
    ///
    /// The offset is restricted between [`Coord::ZERO`] and
    /// [`ListView::max_scroll_offset`].
    #[inline]
    pub fn scroll_offset(&self) -> Coord {
        self.scroll.offset()
    }

    /// Set the scroll offset
    ///
    /// The offset is clamped to the available scroll range.
    /// Returns [`TkAction::None`] if the offset is identical to the old offset,
    /// or a greater action if not identical.
    #[inline]
    pub fn set_scroll_offset(&mut self, offset: Coord) -> TkAction {
        self.scroll.set_offset(offset)
    }

    fn allocate(&mut self, number: usize) {
        let number = number.min(self.data.len());
        self.widgets.reserve(number);
        for i in self.widgets.len()..number {
            self.widgets.push(W::new(self.data.get(i)));
        }
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
    fn len(&self) -> usize {
        self.widgets.len()
    }
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn WidgetConfig> {
        self.widgets.get(index).map(|w| w.as_widget())
    }
    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
        self.widgets.get_mut(index).map(|w| w.as_widget_mut())
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> Layout for ListView<D, A, W>
where
    A::Item: Default,
{
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
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
            self.child_inter_margin = rules.margins().0 as u32 + rules.margins().1 as u32;
            rules.multiply_with_margin(2, self.ideal_visible);
            rules.set_stretch(rules.stretch().max(StretchPolicy::HighUtility));
        }
        rules
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, mut align: AlignHints) {
        self.core.rect = rect;

        let data_len = u32::try_from(self.data.len()).unwrap();
        let mut pos = rect.pos;
        let mut child_size = rect.size;
        let content_size;
        let mut skip: Coord;
        let num_visible;
        if self.direction.is_horizontal() {
            if child_size.0 >= self.ideal_visible * self.child_size_ideal {
                child_size.0 = self.child_size_ideal;
            } else {
                child_size.0 = self.child_size_min;
            }
            skip = Size(child_size.0 + self.child_inter_margin, 0).into();
            align.horiz = None;
            num_visible = rect.size.0 / (child_size.0 + self.child_inter_margin) + 2;

            let full_width =
                (child_size.0 + self.child_inter_margin) * data_len - self.child_inter_margin;
            content_size = Size(full_width, child_size.1);
        } else {
            if child_size.1 >= self.ideal_visible * self.child_size_ideal {
                child_size.1 = self.child_size_ideal;
            } else {
                child_size.1 = self.child_size_min;
            }
            skip = Size(0, child_size.1 + self.child_inter_margin).into();
            align.vert = None;
            num_visible = rect.size.1 / (child_size.1 + self.child_inter_margin) + 2;

            let full_height =
                (child_size.1 + self.child_inter_margin) * data_len - self.child_inter_margin;
            content_size = Size(child_size.0, full_height);
        }

        if self.direction.is_reversed() {
            pos += rect.size - skip.into();
            skip = Coord::ZERO - skip;
        }

        let old_len = self.widgets.len();
        self.allocate(num_visible as usize);
        for child in &mut self.widgets[old_len..] {
            // We must solve size rules on new widgets:
            solve_size_rules(child, size_handle, Some(child_size.0), Some(child_size.1));
        }
        for child in self.widgets.iter_mut() {
            child.set_rect(size_handle, Rect::new(pos, child_size), align);
            pos += skip;
        }

        let _ = self.scroll.set_sizes(rect.size, content_size);
    }

    fn spatial_range(&self) -> (usize, usize) {
        let last = WidgetChildren::len(self).wrapping_sub(1);
        match self.direction.is_reversed() {
            false => (0, last),
            true => (last, 0),
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }

        // FIXME: find child
        Some(self.id())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        draw_handle.clip_region(
            self.core.rect,
            self.scroll_offset(),
            ClipRegion::Scroll,
            &mut |draw_handle| {
                for child in self.widgets.iter() {
                    child.draw(draw_handle, mgr, disabled)
                }
            },
        );
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
                for child in &mut self.widgets {
                    if id <= child.id() {
                        let event = self.scroll.offset_event(event);
                        break 'outer child.send(mgr, id, event);
                    }
                }
                return Response::Unhandled(event);
            };
            match response {
                Response::Unhandled(event) => event,
                Response::Focus(rect) => {
                    let (rect, action) = self.scroll.focus_rect(rect, self.core.rect.pos);
                    // TODO: update widgets
                    *mgr += action;
                    return Response::Focus(rect);
                }
                r => return r,
            }
        } else {
            event
        };

        let id = self.id();
        let (action, response) = self.scroll.scroll_by_event(event, |source, _, coord| {
            if source.is_primary() {
                let icon = Some(CursorIcon::Grabbing);
                mgr.request_grab(id, source, coord, GrabMode::Grab, icon);
            }
        });
        if action != TkAction::None {
            // TODO: update widgets
            *mgr += action;
        }
        response.void_into()
    }
}
