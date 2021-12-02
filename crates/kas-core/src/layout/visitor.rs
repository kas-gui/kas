// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout visitor

use super::{AlignHints, AxisInfo, RulesSetter, RulesSolver, SizeRules, Storage};
use super::{RowSetter, RowSolver, RowStorage};
use crate::draw::SizeHandle;
use crate::event::Manager;
use crate::geom::Rect;
use crate::{dir::Directional, WidgetConfig};
use std::iter::ExactSizeIterator;

/// Chaining layout storage
///
/// We support embedded layouts within a single widget which means that we must
/// support storage for multiple layouts, though commonly zero or one layout is
/// used. We therefore use a simple linked list.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[derive(Debug)]
pub struct StorageChain(Option<(Box<StorageChain>, Box<dyn Storage>)>);

impl Default for StorageChain {
    fn default() -> Self {
        StorageChain(None)
    }
}

impl StorageChain {
    /// Access layout storage
    ///
    /// This storage is allocated and initialised on first access.
    ///
    /// Panics if the type `T` differs from the initial usage.
    pub fn storage<T: Storage + Default>(&mut self) -> (&mut T, &mut StorageChain) {
        if let StorageChain(Some(ref mut b)) = self {
            let storage =
                b.1.downcast_mut()
                    .unwrap_or_else(|| panic!("StorageChain::storage::<T>(): incorrect type T"));
            return (storage, &mut b.0);
        }
        // TODO(rust#42877): store (StorageChain, dyn Storage) tuple in a single box
        let s = Box::new(StorageChain(None));
        let t: Box<dyn Storage> = Box::new(T::default());
        *self = StorageChain(Some((s, t)));
        match self {
            StorageChain(Some(b)) => (b.1.downcast_mut::<T>().unwrap(), &mut b.0),
            _ => unreachable!(),
        }
    }
}

/// Implementation helper for layout of children
trait Visitor {
    /// Get size rules for the given axis
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules;

    /// Apply a given `rect` to self
    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints);

    fn is_reversed(&mut self) -> bool;
}

/// A layout visitor
///
/// This constitutes a "visitor" which iterates over each child widget. Layout
/// algorithm details are implemented over this visitor.
pub struct Layout<'a> {
    layout: LayoutType<'a>,
    hints: AlignHints,
}

/// Items which can be placed in a layout
enum LayoutType<'a> {
    /// No layout
    None,
    /// A single child widget
    Single(&'a mut dyn WidgetConfig),
    /// An embedded layout
    // TODO: why use a trait instead of enumerating all options?
    Visitor(Box<dyn Visitor + 'a>), // TODO: inline storage?
}

/// Implement row/column layout for children
struct List<'a, S, D, I> {
    data: &'a mut S,
    direction: D,
    children: I,
}

impl<'a> Default for Layout<'a> {
    fn default() -> Self {
        Layout::none()
    }
}

impl<'a> Layout<'a> {
    /// Construct an empty layout
    pub fn none() -> Self {
        let layout = LayoutType::None;
        let hints = AlignHints::NONE;
        Layout { layout, hints }
    }

    /// Construct a single-item layout
    pub fn single(widget: &'a mut dyn WidgetConfig, hints: AlignHints) -> Self {
        let layout = LayoutType::Single(widget);
        Layout { layout, hints }
    }

    /// Construct a list layout using an iterator over sub-layouts
    pub fn list<I, D, S>(list: I, direction: D, data: &'a mut S, hints: AlignHints) -> Self
    where
        I: ExactSizeIterator<Item = Layout<'a>> + 'a,
        D: Directional,
        S: RowStorage,
    {
        let layout = LayoutType::Visitor(Box::new(List {
            data,
            direction,
            children: list,
        }));
        Layout { layout, hints }
    }

    /// Get size rules for the given axis
    pub fn size_rules(mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        match &mut self.layout {
            LayoutType::None => SizeRules::EMPTY,
            LayoutType::Single(child) => child.size_rules(sh, axis),
            LayoutType::Visitor(visitor) => visitor.size_rules(sh, axis),
        }
    }

    /// Apply a given `rect` to self
    pub fn set_rect(mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let align = self.hints.combine(align);
        match &mut self.layout {
            LayoutType::None => (),
            LayoutType::Single(child) => child.set_rect(mgr, rect, align),
            LayoutType::Visitor(layout) => layout.set_rect(mgr, rect, align),
        }
    }

    /// Return true if layout is up/left
    ///
    /// This is a lazy method of implementing tab order for reversible layouts.
    pub fn is_reversed(mut self) -> bool {
        match &mut self.layout {
            LayoutType::None => false,
            LayoutType::Single(_) => false,
            LayoutType::Visitor(layout) => layout.is_reversed(),
        }
    }
}

impl<'a, S: RowStorage, D: Directional, I> Visitor for List<'a, S, D, I>
where
    I: ExactSizeIterator<Item = Layout<'a>>,
{
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in (&mut self.children).enumerate() {
            solver.for_child(self.data, n, |axis| child.size_rules(sh, axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, self.data);

        for (n, child) in (&mut self.children).enumerate() {
            child.set_rect(mgr, setter.child_rect(self.data, n), align);
        }
    }

    fn is_reversed(&mut self) -> bool {
        self.direction.is_reversed()
    }
}
