// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view widget

use super::{Accessor, DefaultView, ViewWidget};
use kas::prelude::*;

/// List view widget
#[handler(send=noauto, msg=<W as Handler>::Msg)]
#[widget(children=noauto)]
#[derive(Clone, Default, Debug, Widget)]
pub struct ListView<
    D: Directional,
    A: Accessor<usize>,
    W: ViewWidget<A::Item> = <<A as Accessor<usize>>::Item as DefaultView>::Widget,
> {
    first_id: WidgetId,
    #[widget_core]
    core: CoreData,
    data: A,
    widgets: Vec<W>,
    direction: D,
    child_size_min: u32,
    child_size_ideal: u32,
    child_inter_margin: i32,
}

impl<D: Directional + Default, A: Accessor<usize>, W: ViewWidget<A::Item>> ListView<D, A, W> {
    /// Construct a new instance
    ///
    /// This constructor is available where the direction is determined by the
    /// type: for `D: Directional + Default`. In other cases, use
    /// [`List::new_with_direction`].
    pub fn new(data: A) -> Self {
        ListView {
            first_id: Default::default(),
            core: Default::default(),
            data,
            widgets: Default::default(),
            direction: Default::default(),
            child_size_min: 0,
            child_size_ideal: 0,
            child_inter_margin: 0,
        }
    }
}
impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> ListView<D, A, W> {
    /// Construct a new instance with explicit direction
    pub fn new_with_direction(direction: D, data: A) -> Self {
        ListView {
            first_id: Default::default(),
            core: Default::default(),
            data,
            widgets: Default::default(),
            direction,
            child_size_min: 0,
            child_size_ideal: 0,
            child_inter_margin: 0,
        }
    }

    /// Get the direction of contents
    pub fn direction(&self) -> Direction {
        self.direction.as_direction()
    }

    fn allocate(&mut self, number: usize) {
        self.widgets.reserve(number);
        let len = self.data.len();
        for i in self.widgets.len()..number {
            self.widgets.push(if i < len {
                W::new(self.data.get(i))
            } else {
                W::default()
            });
        }
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> WidgetChildren
    for ListView<D, A, W>
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

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> Layout for ListView<D, A, W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        // TODO: when and how to allocate children?
        if self.widgets.len() < 1 {
            self.allocate(1);
        }
        let mut rules = self.widgets[0].size_rules(size_handle, axis);
        if axis.is_vertical() == self.direction.is_vertical() {
            self.child_size_min = rules.min_size();
            self.child_size_ideal = rules.ideal_size();
            self.child_inter_margin = rules.margins().0 as i32 + rules.margins().1 as i32;
            rules = SizeRules::new(
                rules.min_size(),
                rules.appended(rules).appended(rules).ideal_size(),
                rules.margins(),
                rules.stretch().max(StretchPolicy::HighUtility),
            );
        }
        rules
    }

    fn set_rect(&mut self, rect: Rect, mut align: AlignHints) {
        self.core.rect = rect;

        let mut pos = rect.pos;
        let mut child_size = rect.size;
        let mut skip;
        if self.direction.is_horizontal() {
            if child_size.0 >= 3 * self.child_size_ideal {
                child_size.0 = self.child_size_ideal;
            } else {
                child_size.0 = self.child_size_min;
            }
            skip = Coord(child_size.0 as i32 + self.child_inter_margin, 0);
            align.horiz = None;
        } else {
            if child_size.1 >= 3 * self.child_size_ideal {
                child_size.1 = self.child_size_ideal;
            } else {
                child_size.1 = self.child_size_min;
            }
            skip = Coord(0, child_size.0 as i32 + self.child_inter_margin);
            align.vert = None;
        }

        if self.direction.is_reversed() {
            pos += rect.size - skip.into();
            skip = Coord::ZERO - skip;
        }

        for child in self.widgets.iter_mut() {
            child.set_rect(Rect::new(pos, child_size), align);
            pos += skip;
        }
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
        // FIXME: cull invisible children and restrict draw region
        for child in self.widgets.iter() {
            child.draw(draw_handle, mgr, disabled)
        }
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> SendEvent for ListView<D, A, W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if !self.is_disabled() {
            if id == self.id() {
                return self.handle(mgr, event);
            }
            for child in &mut self.widgets {
                if id <= child.id() {
                    return child.send(mgr, id, event);
                }
            }
        }

        Response::Unhandled(event)
    }
}
