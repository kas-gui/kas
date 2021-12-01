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
pub trait Visitor {
    /// Get size rules for the given axis
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules;

    /// Apply a given `rect` to self
    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints);
}

/// Items which can be placed in a layout
pub enum Item<'a> {
    /// A widget
    Widget(&'a mut dyn WidgetConfig),
    /// An embedded layout
    Layout(Box<dyn Visitor + 'a>), // TODO: inline storage?
}

/// Implement row/column layout for children
pub struct List<'a, S, D, I> {
    data: &'a mut S,
    direction: D,
    children: I,
}

impl<'a, S: RowStorage, D: Directional, I> List<'a, S, D, I>
where
    I: ExactSizeIterator<Item = (Item<'a>, AlignHints)>,
{
    /// Construct
    ///
    /// -   `data`: associated storage type
    /// -   `direction`: row/column direction
    /// -   `children`: iterator over `(hints, item)` tuples where
    ///     `hints` is optional alignment hints and
    ///     `item` is a layout [`Item`].
    pub fn new(data: &'a mut S, direction: D, children: I) -> Self {
        List {
            data,
            direction,
            children,
        }
    }
}

impl<'a, S: RowStorage, D: Directional, I> Visitor for List<'a, S, D, I>
where
    I: ExactSizeIterator<Item = (Item<'a>, AlignHints)>,
{
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, (child, _)) in (&mut self.children).enumerate() {
            match child {
                Item::Widget(child) => {
                    solver.for_child(self.data, n, |axis| child.size_rules(sh, axis))
                }
                Item::Layout(mut layout) => {
                    solver.for_child(self.data, n, |axis| layout.size_rules(sh, axis))
                }
            }
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, self.data);

        for (n, (child, hints)) in (&mut self.children).enumerate() {
            let align = hints.combine(align);
            match child {
                Item::Widget(child) => child.set_rect(mgr, setter.child_rect(self.data, n), align),
                Item::Layout(mut layout) => {
                    layout.set_rect(mgr, setter.child_rect(self.data, n), align)
                }
            }
        }
    }
}
