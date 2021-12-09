// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A stack

use std::fmt::Debug;
use std::ops::{Index, IndexMut};

use kas::{event, prelude::*};

/// A stack of boxed widgets
///
/// This is a parametrisation of [`Stack`].
pub type BoxStack<M> = Stack<Box<dyn Widget<Msg = M>>>;

/// A stack of widget references
///
/// This is a parametrisation of [`Stack`].
pub type RefStack<'a, M> = Stack<&'a mut dyn Widget<Msg = M>>;

widget! {
    /// A stack of widgets
    ///
    /// A stack consists a set of child widgets, all of equal size.
    /// Only a single member is visible at a time.
    ///
    /// This may only be parametrised with a single widget type; [`BoxStack`] is
    /// a parametrisation allowing run-time polymorphism of child widgets.
    ///
    /// Configuring and resizing elements is O(n) in the number of children.
    /// Drawing and event handling is O(1).
    #[derive(Clone, Default, Debug)]
    #[handler(msg=<W as event::Handler>::Msg)]
    pub struct Stack<W: Widget> {
        #[widget_core]
        core: CoreData,
        widgets: Vec<W>,
        active: usize,
    }

    impl WidgetChildren for Self {
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

    impl Layout for Self {
        fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
            let mut rules = SizeRules::EMPTY;
            for child in &mut self.widgets {
                rules = rules.max(child.size_rules(size_handle, axis));
            }
            rules
        }

        fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            for child in &mut self.widgets {
                child.set_rect(mgr, rect, align);
            }
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if self.active < self.widgets.len() {
                return self.widgets[self.active].find_id(coord);
            }
            None
        }

        fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            let disabled = disabled || self.is_disabled();
            if self.active < self.widgets.len() {
                self.widgets[self.active].draw(draw, mgr, disabled);
            }
        }
    }

    impl event::SendEvent for Self {
        fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if !self.is_disabled() {
                if let Some(index) = self.id().index_of_child(id) {
                    if let Some(child) = self.widgets.get_mut(index) {
                        return match child.send(mgr, id, event) {
                            Response::Focus(rect) => {
                                *mgr |= self.set_active(index);
                                Response::Focus(rect)
                            }
                            r => r,
                        };
                    }
                }
            }

            Response::Unhandled
        }
    }

    impl Index<usize> for Self {
        type Output = W;

        fn index(&self, index: usize) -> &Self::Output {
            &self.widgets[index]
        }
    }

    impl IndexMut<usize> for Self {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            &mut self.widgets[index]
        }
    }
}

impl<W: Widget> Stack<W> {
    /// Construct a new instance
    ///
    /// If `active < widgets.len()`, then `widgets[active]` will initially be
    /// visible; otherwise, no widget will be visible.
    pub fn new(widgets: Vec<W>, active: usize) -> Self {
        Stack {
            core: Default::default(),
            widgets,
            active,
        }
    }

    /// Get the index of the active widget
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Change the active widget via index
    ///
    /// It is not required that `active < self.len()`; if not, no widget will be
    /// drawn or respond to events, but the stack will still size as required by
    /// child widgets.
    pub fn set_active(&mut self, active: usize) -> TkAction {
        if self.active == active {
            TkAction::empty()
        } else {
            self.active = active;
            TkAction::REGION_MOVED
        }
    }

    /// Get a direct reference to the active widget, if any
    pub fn active(&self) -> Option<&W> {
        if self.active < self.widgets.len() {
            Some(&self.widgets[self.active])
        } else {
            None
        }
    }

    /// Get a direct mutable reference to the active widget, if any
    pub fn active_mut(&mut self) -> Option<&mut W> {
        if self.active < self.widgets.len() {
            Some(&mut self.widgets[self.active])
        } else {
            None
        }
    }

    /// True if there are no child widgets
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Returns the number of child widgets
    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    /// Returns the number of elements the vector can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.widgets.capacity()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// into the list. See documentation of [`Vec::reserve`].
    pub fn reserve(&mut self, additional: usize) {
        self.widgets.reserve(additional);
    }

    /// Remove all child widgets
    ///
    /// Triggers a [reconfigure action](Manager::send_action) if any widget is
    /// removed.
    pub fn clear(&mut self) -> TkAction {
        let action = match self.widgets.is_empty() {
            true => TkAction::empty(),
            false => TkAction::RECONFIGURE,
        };
        self.widgets.clear();
        action
    }

    /// Append a child widget
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn push(&mut self, widget: W) -> TkAction {
        self.widgets.push(widget);
        TkAction::RECONFIGURE
    }

    /// Remove the last child widget
    ///
    /// Returns `None` if there are no children. Otherwise, this
    /// triggers a reconfigure before the next draw operation.
    ///
    /// Triggers a [reconfigure action](Manager::send_action) if any widget is
    /// removed.
    pub fn pop(&mut self) -> (Option<W>, TkAction) {
        let action = match self.widgets.is_empty() {
            true => TkAction::empty(),
            false => TkAction::RECONFIGURE,
        };
        (self.widgets.pop(), action)
    }

    /// Inserts a child widget position `index`
    ///
    /// Panics if `index > len`.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn insert(&mut self, index: usize, widget: W) -> TkAction {
        self.widgets.insert(index, widget);
        TkAction::RECONFIGURE
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn remove(&mut self, index: usize) -> (W, TkAction) {
        let r = self.widgets.remove(index);
        (r, TkAction::RECONFIGURE)
    }

    /// Replace the child at `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    // TODO: in theory it is possible to avoid a reconfigure where both widgets
    // have no children and have compatible size. Is this a good idea and can
    // we somehow test "has compatible size"?
    pub fn replace(&mut self, index: usize, mut widget: W) -> (W, TkAction) {
        std::mem::swap(&mut widget, &mut self.widgets[index]);
        (widget, TkAction::RECONFIGURE)
    }

    /// Append child widgets from an iterator
    ///
    /// Triggers a [reconfigure action](Manager::send_action) if any widgets
    /// are added.
    pub fn extend<T: IntoIterator<Item = W>>(&mut self, iter: T) -> TkAction {
        let len = self.widgets.len();
        self.widgets.extend(iter);
        match len == self.widgets.len() {
            true => TkAction::empty(),
            false => TkAction::RECONFIGURE,
        }
    }

    /// Resize, using the given closure to construct new widgets
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn resize_with<F: Fn(usize) -> W>(&mut self, len: usize, f: F) -> TkAction {
        let l0 = self.widgets.len();
        if l0 == len {
            return TkAction::empty();
        } else if l0 > len {
            self.widgets.truncate(len);
        } else {
            self.widgets.reserve(len);
            for i in l0..len {
                self.widgets.push(f(i));
            }
        }
        TkAction::RECONFIGURE
    }

    /// Retain only widgets satisfying predicate `f`
    ///
    /// See documentation of [`Vec::retain`].
    ///
    /// Triggers a [reconfigure action](Manager::send_action) if any widgets
    /// are removed.
    pub fn retain<F: FnMut(&W) -> bool>(&mut self, f: F) -> TkAction {
        let len = self.widgets.len();
        self.widgets.retain(f);
        match len == self.widgets.len() {
            true => TkAction::empty(),
            false => TkAction::RECONFIGURE,
        }
    }
}
