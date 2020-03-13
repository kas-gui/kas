// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;

use super::{AxisInfo, RowStorage, RowTemp, RulesSetter, RulesSolver, SizeRules};
use crate::geom::{Coord, Rect};
use crate::{Directional, Widget};

/// A [`RulesSolver`] for rows (and, without loss of generality, for columns).
///
/// This is parameterised over:
///
/// -   `T:` [`RowTemp`] — temporary storage type
/// -   `S:` [`RowStorage`] — persistent storage type
pub struct RowSolver<T: RowTemp, S: RowStorage> {
    // Generalisation implies that axis.vert() is incorrect
    axis: AxisInfo,
    axis_is_vertical: bool,
    rules: SizeRules,
    widths: T,
    _s: PhantomData<S>,
}

impl<T: RowTemp, S: RowStorage> RowSolver<T, S> {
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `dim`: direction and number of items
    /// - `storage`: reference to persistent storage
    pub fn new<D: Directional>(axis: AxisInfo, dim: (D, usize), storage: &mut S) -> Self {
        let mut widths = T::default();
        widths.set_len(dim.1);
        assert!(widths.as_ref().iter().all(|w| *w == 0));
        storage.set_len(dim.1 + 1);

        let axis_is_vertical = axis.is_vertical() ^ dim.0.is_vertical();

        if axis.has_fixed && axis_is_vertical {
            // TODO: cache this for use by set_rect?
            SizeRules::solve_seq(widths.as_mut(), storage.as_ref(), axis.other_axis);
        }

        RowSolver {
            axis,
            axis_is_vertical,
            rules: SizeRules::EMPTY,
            widths,
            _s: Default::default(),
        }
    }
}

impl<T: RowTemp, S: RowStorage> RulesSolver for RowSolver<T, S> {
    type Storage = S;
    type ChildInfo = usize;

    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        storage: &mut Self::Storage,
        child_info: Self::ChildInfo,
        child_rules: CR,
    ) {
        if self.axis.has_fixed && self.axis_is_vertical {
            self.axis.other_axis = self.widths.as_ref()[child_info];
        }
        let child_rules = child_rules(self.axis);
        if !self.axis_is_vertical {
            storage.as_mut()[child_info] = child_rules;
            self.rules.append(child_rules);
        } else {
            self.rules = self.rules.max(child_rules);
        }
    }

    fn finish(self, storage: &mut Self::Storage) -> SizeRules {
        let cols = storage.as_ref().len() - 1;
        if !self.axis_is_vertical {
            storage.as_mut()[cols] = self.rules;
        }

        self.rules
    }
}

/// A [`RulesSetter`] for rows (and, without loss of generality, for columns).
///
/// This is parameterised over:
///
/// -   `D:` [`Directional`] — whether this represents a row or a column
/// -   `T:` [`RowTemp`] — temporary storage type
/// -   `S:` [`RowStorage`] — persistent storage type
pub struct RowSetter<D, T: RowTemp, S: RowStorage> {
    crect: Rect,
    widths: T,
    offsets: T,
    direction: D,
    _s: PhantomData<S>,
}

impl<D: Directional, T: RowTemp, S: RowStorage> RowSetter<D, T, S> {
    pub fn new(rect: Rect, dim: (D, usize), storage: &mut S) -> Self {
        let mut widths = T::default();
        widths.set_len(dim.1);
        let mut offsets = T::default();
        offsets.set_len(dim.1);
        storage.set_len(dim.1 + 1);

        let (pos, width) = match dim.0.is_horizontal() {
            true => (rect.pos.0, rect.size.0),
            false => (rect.pos.1, rect.size.1),
        };

        SizeRules::solve_seq(widths.as_mut(), storage.as_ref(), width);
        offsets.as_mut()[0] = pos as u32;
        for i in 1..offsets.as_ref().len() {
            let i1 = i - 1;
            let m1 = storage.as_ref()[i1].margins().1;
            let m0 = storage.as_ref()[i].margins().0;
            offsets.as_mut()[i] = offsets.as_ref()[i1] + widths.as_ref()[i1] + m1.max(m0) as u32;
        }

        RowSetter {
            crect: rect,
            widths,
            offsets,
            direction: dim.0,
            _s: Default::default(),
        }
    }
}

impl<D: Directional, T: RowTemp, S: RowStorage> RulesSetter for RowSetter<D, T, S> {
    type Storage = S;
    type ChildInfo = usize;

    fn child_rect(&mut self, index: Self::ChildInfo) -> Rect {
        if self.direction.is_horizontal() {
            self.crect.pos.0 = self.offsets.as_ref()[index] as i32;
            self.crect.size.0 = self.widths.as_ref()[index];
        } else {
            self.crect.pos.1 = self.offsets.as_ref()[index] as i32;
            self.crect.size.1 = self.widths.as_ref()[index];
        }
        self.crect
    }
}

/// Allows efficient implementations of `draw` / event handlers based on the
/// layout representation.
///
/// This is only applicable where child widgets are contained in a slice of type
/// `W: Widget` (which may be `Box<dyn Widget>`). In other cases, the naive
/// implementation (test all items) must be used.
#[derive(Clone, Copy, Debug)]
pub struct RowPositionSolver<D: Directional> {
    direction: D,
}

impl<D: Directional> RowPositionSolver<D> {
    /// Construct with given directionality
    pub fn new(direction: D) -> Self {
        RowPositionSolver { direction }
    }

    fn binary_search<W: Widget>(self, widgets: &[W], coord: Coord) -> Result<usize, usize> {
        if self.direction.is_horizontal() {
            widgets.binary_search_by_key(&coord.0, |w| w.rect().pos.0)
        } else {
            widgets.binary_search_by_key(&coord.1, |w| w.rect().pos.1)
        }
    }

    /// Find the child containing the given coordinates
    ///
    /// Returns `None` when the coordinates lie within the margin area or
    /// outside of the parent widget.
    pub fn find_child<'a, W: Widget>(self, widgets: &'a [W], coord: Coord) -> Option<&'a W> {
        let index = match self.binary_search(widgets, coord) {
            Ok(i) => i,
            Err(i) => {
                if i == 0 || !widgets[i - 1].rect().contains(coord) {
                    return None;
                }
                i - 1
            }
        };
        Some(&widgets[index])
    }

    /// Call `f` on each child intersecting the given `rect`
    pub fn for_children<W: Widget, F: FnMut(&W)>(self, widgets: &[W], rect: Rect, mut f: F) {
        let start = match self.binary_search(widgets, rect.pos) {
            Ok(i) => i,
            Err(i) if i > 0 => {
                let j = i - 1;
                if widgets[j].rect().contains(rect.pos) {
                    j
                } else {
                    i
                }
            }
            Err(_) => 0,
        };

        let end = rect.pos + Coord::from(rect.size);

        for i in start..widgets.len() {
            let child = &widgets[i];
            if self.direction.is_horizontal() {
                if child.rect().pos.0 >= end.0 {
                    break;
                }
            } else {
                if child.rect().pos.1 >= end.1 {
                    break;
                }
            }
            f(child);
        }
    }
}
