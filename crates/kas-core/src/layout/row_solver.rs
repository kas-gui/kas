// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;
use std::ops::Range;

use super::{AxisInfo, SizeRules};
use super::{RowStorage, RowTemp, RulesSetter, RulesSolver};
use crate::dir::{Direction, Directional};
use crate::geom::{Coord, Rect};
use crate::{Collection, Layout};

/// A [`RulesSolver`] for rows (and, without loss of generality, for columns).
///
/// This is parameterised over:
///
/// -   `S:` [`RowStorage`] — persistent storage type
pub struct RowSolver<S: RowStorage> {
    // Generalisation implies that axis.vert() is incorrect
    axis: AxisInfo,
    axis_is_vertical: bool,
    axis_is_reversed: bool,
    rules: Option<SizeRules>,
    _s: PhantomData<S>,
}

impl<S: RowStorage> RowSolver<S> {
    /// Construct.
    ///
    /// Argument order is consistent with other [`RulesSolver`]s.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `(dir, len)`: direction and number of items
    /// - `storage`: reference to persistent storage
    pub fn new<D: Directional>(axis: AxisInfo, (dir, len): (D, usize), storage: &mut S) -> Self {
        storage.set_dim(len);

        let axis_is_vertical = axis.is_vertical() ^ dir.is_vertical();

        if axis.has_fixed && axis_is_vertical {
            let (widths, rules) = storage.widths_and_rules();
            SizeRules::solve_seq(widths, rules, axis.other_axis);
        }

        RowSolver {
            axis,
            axis_is_vertical,
            axis_is_reversed: dir.is_reversed(),
            rules: None,
            _s: Default::default(),
        }
    }
}

impl<S: RowStorage> RulesSolver for RowSolver<S> {
    type Storage = S;
    type ChildInfo = usize;

    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        storage: &mut Self::Storage,
        index: Self::ChildInfo,
        child_rules: CR,
    ) {
        if self.axis.has_fixed && self.axis_is_vertical {
            self.axis.other_axis = storage.widths()[index];
        }
        let child_rules = child_rules(self.axis);

        if !self.axis_is_vertical {
            storage.rules()[index] = child_rules;
            if let Some(rules) = self.rules {
                if self.axis_is_reversed {
                    self.rules = Some(child_rules.appended(rules));
                } else {
                    self.rules = Some(rules.appended(child_rules));
                }
            } else {
                self.rules = Some(child_rules);
            }
        } else {
            self.rules = Some(
                self.rules
                    .map(|rules| rules.max(child_rules))
                    .unwrap_or(child_rules),
            );
        }
    }

    fn finish(self, _: &mut Self::Storage) -> SizeRules {
        self.rules.unwrap_or(SizeRules::EMPTY)
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
    rect: Rect,
    offsets: T,
    direction: D,
    _s: PhantomData<S>,
}

impl<D: Directional, T: RowTemp, S: RowStorage> RowSetter<D, T, S> {
    /// Construct
    ///
    /// Argument order is consistent with other [`RulesSetter`]s.
    ///
    /// -   `rect`: the [`Rect`] within which to position children
    /// - `(direction, len)`: direction and number of items
    /// -   `storage`: access to the solver's storage
    pub fn new(rect: Rect, (direction, len): (D, usize), storage: &mut S) -> Self {
        let mut offsets = T::default();
        offsets.set_len(len);
        storage.set_dim(len);

        if len > 0 {
            let is_horiz = direction.is_horizontal();
            let width = if is_horiz { rect.size.0 } else { rect.size.1 };
            let (widths, rules) = storage.widths_and_rules();
            SizeRules::solve_seq(widths, rules, width);
        }

        let _s = Default::default();
        let mut row = RowSetter {
            rect,
            offsets,
            direction,
            _s,
        };
        row.update_offsets(storage);
        row
    }

    /// Construct without solving
    ///
    /// In this case, it is assumed that the storage was already solved by a
    /// previous `RowSetter`. The user should optionally call `solve_range` on
    /// any ranges needing updating and finally call `update_offsets` before
    /// using this `RowSetter` to calculate child positions.
    pub fn new_unsolved(rect: Rect, (direction, len): (D, usize), storage: &mut S) -> Self {
        let mut offsets = T::default();
        offsets.set_len(len);
        storage.set_dim(len);

        let _s = Default::default();
        RowSetter {
            rect,
            offsets,
            direction,
            _s,
        }
    }

    pub fn update_offsets(&mut self, storage: &mut S) {
        let offsets = self.offsets.as_mut();
        let len = offsets.len();
        if len == 0 {
            return;
        }

        let pos = if self.direction.is_horizontal() {
            self.rect.pos.0
        } else {
            self.rect.pos.1
        };

        if self.direction.is_reversed() {
            offsets[len - 1] = pos;
            for i in (0..(len - 1)).rev() {
                let i1 = i + 1;
                let m1 = storage.rules()[i1].margins_i32().1;
                let m0 = storage.rules()[i].margins_i32().0;
                offsets[i] = offsets[i1] + storage.widths()[i1] + m1.max(m0);
            }
        } else {
            offsets[0] = pos;
            for i in 1..len {
                let i1 = i - 1;
                let m1 = storage.rules()[i1].margins_i32().1;
                let m0 = storage.rules()[i].margins_i32().0;
                offsets[i] = offsets[i1] + storage.widths()[i1] + m1.max(m0);
            }
        }
    }

    pub fn solve_range(&mut self, storage: &mut S, range: Range<usize>, width: i32) {
        assert!(range.end <= self.offsets.as_mut().len());

        let (widths, rules) = storage.widths_and_rules();
        SizeRules::solve_seq(&mut widths[range.clone()], &rules[range], width);
    }
}

impl<D: Directional, T: RowTemp, S: RowStorage> RulesSetter for RowSetter<D, T, S> {
    type Storage = S;
    type ChildInfo = usize;

    fn child_rect(&mut self, storage: &mut Self::Storage, index: Self::ChildInfo) -> Rect {
        let mut rect = self.rect;
        if self.direction.is_horizontal() {
            rect.pos.0 = self.offsets.as_mut()[index];
            rect.size.0 = storage.widths()[index];
        } else {
            rect.pos.1 = self.offsets.as_mut()[index];
            rect.size.1 = storage.widths()[index];
        }
        rect
    }

    fn maximal_rect_of(&mut self, storage: &mut Self::Storage, index: Self::ChildInfo) -> Rect {
        let pre_rules = SizeRules::min_sum(&storage.rules()[0..index]);
        let m = storage.rules()[index].margins();
        let len = storage.widths().len();
        let post_rules = SizeRules::min_sum(&storage.rules()[(index + 1)..len]);

        let size1 = pre_rules.min_size() + i32::from(pre_rules.margins().1.max(m.0));
        let size2 = size1 + post_rules.min_size() + i32::from(post_rules.margins().0.max(m.1));

        let mut rect = self.rect;
        if self.direction.is_horizontal() {
            rect.pos.0 = self.rect.pos.0 + size1;
            rect.size.0 = (self.rect.size.0 - size2).max(0);
        } else {
            rect.pos.1 = self.rect.pos.1 + size1;
            rect.size.1 = (self.rect.size.1 - size2).max(0);
        }
        rect
    }
}

/// Allows efficient implementations of `draw` / event handlers based on the
/// layout representation.
///
/// This is only applicable where child widgets are contained in a [`Collection`].
#[derive(Clone, Copy, Debug)]
pub struct RowPositionSolver<D: Directional> {
    direction: D,
}

impl<D: Directional> RowPositionSolver<D> {
    /// Construct with given directionality
    pub fn new(direction: D) -> Self {
        RowPositionSolver { direction }
    }

    fn binary_search<C: Collection + ?Sized>(
        self,
        widgets: &C,
        coord: Coord,
    ) -> Option<Result<usize, usize>> {
        match self.direction.as_direction() {
            Direction::Right => widgets.binary_search_by(|w| w.rect().pos.0.cmp(&coord.0)),
            Direction::Down => widgets.binary_search_by(|w| w.rect().pos.1.cmp(&coord.1)),
            Direction::Left => widgets.binary_search_by(|w| w.rect().pos.0.cmp(&coord.0).reverse()),
            Direction::Up => widgets.binary_search_by(|w| w.rect().pos.1.cmp(&coord.1).reverse()),
        }
    }

    /// Find the child containing the given coordinates
    ///
    /// Returns `None` when the coordinates lie within the margin area or
    /// outside of the parent widget.
    /// Also returns `None` if [`Collection::get_layout`] returns `None` for
    /// some index less than `len` (a theoretical but unexpected error case).
    pub fn find_child_index<C: Collection + ?Sized>(
        self,
        widgets: &C,
        coord: Coord,
    ) -> Option<usize> {
        match self.binary_search(widgets, coord)? {
            Ok(i) => Some(i),
            Err(i) if self.direction.is_reversed() => {
                if i == widgets.len() || !widgets.get_layout(i)?.rect().contains(coord) {
                    None
                } else {
                    Some(i)
                }
            }
            Err(i) => {
                if i == 0 || !widgets.get_layout(i - 1)?.rect().contains(coord) {
                    None
                } else {
                    Some(i - 1)
                }
            }
        }
    }

    /// Find the child containing the given coordinates
    ///
    /// Returns `None` when the coordinates lie within the margin area or
    /// outside of the parent widget.
    /// Also returns `None` if [`Collection::get_layout`] returns `None` for
    /// some index less than `len` (a theoretical but unexpected error case).
    #[inline]
    pub fn find_child<C: Collection + ?Sized>(
        self,
        widgets: &C,
        coord: Coord,
    ) -> Option<&dyn Layout> {
        self.find_child_index(widgets, coord)
            .and_then(|i| widgets.get_layout(i))
    }

    /// Find the child containing the given coordinates
    ///
    /// Returns `None` when the coordinates lie within the margin area or
    /// outside of the parent widget.
    /// Also returns `None` if [`Collection::get_layout`] returns `None` for
    /// some index less than `len` (a theoretical but unexpected error case).
    #[inline]
    pub fn find_child_mut<C: Collection + ?Sized>(
        self,
        widgets: &mut C,
        coord: Coord,
    ) -> Option<&mut dyn Layout> {
        self.find_child_index(widgets, coord)
            .and_then(|i| widgets.get_mut_layout(i))
    }

    /// Call `f` on each child intersecting the given `rect`
    pub fn for_children_mut<C: Collection + ?Sized, F: FnMut(&mut dyn Layout)>(
        self,
        widgets: &mut C,
        rect: Rect,
        mut f: F,
    ) {
        let (pos, end) = match self.direction.is_reversed() {
            false => (rect.pos, rect.pos2()),
            true => (rect.pos2(), rect.pos),
        };
        let start = match self.binary_search(widgets, pos) {
            Some(Ok(i)) => i,
            Some(Err(i)) if i > 0 => {
                let mut j = i - 1;
                if let Some(rect) = widgets.get_layout(j).map(|l| l.rect()) {
                    let cond = match self.direction.as_direction() {
                        Direction::Right => pos.0 < rect.pos2().0,
                        Direction::Down => pos.1 < rect.pos2().1,
                        Direction::Left => rect.pos.0 <= pos.0,
                        Direction::Up => rect.pos.1 <= pos.1,
                    };
                    if !cond {
                        j = i;
                    }
                }
                j
            }
            _ => 0,
        };

        for i in start..widgets.len() {
            if let Some(child) = widgets.get_mut_layout(i) {
                let do_break = match self.direction.as_direction() {
                    Direction::Right => child.rect().pos.0 >= end.0,
                    Direction::Down => child.rect().pos.1 >= end.1,
                    Direction::Left => child.rect().pos2().0 < end.0,
                    Direction::Up => child.rect().pos2().1 < end.1,
                };
                if do_break {
                    break;
                }
                f(child);
            }
        }
    }
}
