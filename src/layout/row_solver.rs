// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;
use std::ops::Range;

use super::{AxisInfo, RowStorage, RowTemp, RulesSetter, RulesSolver, SizeRules};
use crate::geom::{Coord, Rect};
use crate::{Align, AlignHints, Direction, Directional, Widget};

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
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `dim`: direction and number of items
    /// - `storage`: reference to persistent storage
    pub fn new<D: Directional>(axis: AxisInfo, (dir, len): (D, usize), storage: &mut S) -> Self {
        storage.set_dim(len);

        let axis_is_vertical = axis.is_vertical() ^ dir.is_vertical();

        if axis.has_fixed && axis_is_vertical {
            let (rules, widths) = storage.rules_and_widths();
            SizeRules::solve_seq_total(widths, rules, axis.other_axis);
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
        child_info: Self::ChildInfo,
        child_rules: CR,
    ) {
        if self.axis.has_fixed && self.axis_is_vertical {
            self.axis.other_axis = storage.widths()[child_info];
        }
        let child_rules = child_rules(self.axis);
        if !self.axis_is_vertical {
            storage.rules()[child_info] = child_rules;
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

    fn finish(self, storage: &mut Self::Storage) -> SizeRules {
        let cols = storage.rules().len() - 1;
        let rules = self.rules.unwrap_or(SizeRules::EMPTY);
        if !self.axis_is_vertical {
            storage.rules()[cols] = rules;
        }

        rules
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
    /// All setter constructors take the following arguments:
    ///
    /// -   `rect`: the [`Rect`] within which to position children
    /// -   `dim`: dimension information (specific to the setter, in this case
    ///     the direction and number of columns/rows)
    /// -   `align`: alignment hints
    /// -   `storage`: access to the solver's storage
    pub fn new(
        mut rect: Rect,
        (direction, len): (D, usize),
        align: AlignHints,
        storage: &mut S,
    ) -> Self {
        let mut offsets = T::default();
        offsets.set_len(len);
        storage.set_dim(len);

        if len > 0 {
            let is_horiz = direction.is_horizontal();
            let mut width = if is_horiz { rect.size.0 } else { rect.size.1 };
            let (rules, widths) = storage.rules_and_widths();
            let ideal = rules[len].ideal_size();
            let align = if is_horiz { align.horiz } else { align.vert };
            let align = align.unwrap_or(Align::Stretch);
            if align != Align::Stretch && width > ideal {
                let extra = width - ideal;
                width = ideal;
                let offset = match align {
                    Align::Begin | Align::Stretch => 0,
                    Align::Centre => extra / 2,
                    Align::End => extra,
                } as i32;
                if is_horiz {
                    rect.pos.0 += offset;
                } else {
                    rect.pos.1 += offset;
                }
            }
            SizeRules::solve_seq_total(widths, rules, width);
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
    ///
    /// It is also assumed that alignment is [`Align::Stretch`].
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
            offsets[len - 1] = pos as u32;
            for i in (0..(len - 1)).rev() {
                let i1 = i + 1;
                let m1 = storage.rules()[i1].margins().1;
                let m0 = storage.rules()[i].margins().0;
                offsets[i] = offsets[i1] + storage.widths()[i1] + m1.max(m0) as u32;
            }
        } else {
            offsets[0] = pos as u32;
            for i in 1..len {
                let i1 = i - 1;
                let m1 = storage.rules()[i1].margins().1;
                let m0 = storage.rules()[i].margins().0;
                offsets[i] = offsets[i1] + storage.widths()[i1] + m1.max(m0) as u32;
            }
        }
    }

    pub fn solve_range(&mut self, storage: &mut S, range: Range<usize>, width: u32) {
        assert!(range.end <= self.offsets.as_mut().len());

        let (rules, widths) = storage.rules_and_widths();
        SizeRules::solve_seq(&mut widths[range.clone()], &rules[range], width);
    }
}

impl<D: Directional, T: RowTemp, S: RowStorage> RulesSetter for RowSetter<D, T, S> {
    type Storage = S;
    type ChildInfo = usize;

    fn child_rect(&mut self, storage: &mut Self::Storage, index: Self::ChildInfo) -> Rect {
        let mut rect = self.rect;
        if self.direction.is_horizontal() {
            rect.pos.0 = self.offsets.as_mut()[index] as i32;
            rect.size.0 = storage.widths()[index];
        } else {
            rect.pos.1 = self.offsets.as_mut()[index] as i32;
            rect.size.1 = storage.widths()[index];
        }
        rect
    }

    fn maximal_rect_of(&mut self, storage: &mut Self::Storage, index: Self::ChildInfo) -> Rect {
        let pre_rules = SizeRules::min_sum(&storage.rules()[0..index]);
        let m = storage.rules()[index].margins();
        let len = storage.widths().len();
        let post_rules = SizeRules::min_sum(&storage.rules()[(index + 1)..len]);

        let size1 = pre_rules.min_size() as i32 + pre_rules.margins().1.max(m.0) as i32;
        let size2 = size1 as u32 + post_rules.min_size() + post_rules.margins().0.max(m.1) as u32;

        let mut rect = self.rect;
        if self.direction.is_horizontal() {
            rect.pos.0 = self.rect.pos.0 + size1;
            rect.size.0 = self.rect.size.0.saturating_sub(size2);
        } else {
            rect.pos.1 = self.rect.pos.1 + size1;
            rect.size.1 = self.rect.size.1.saturating_sub(size2);
        }
        rect
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
        match self.direction.as_direction() {
            Direction::Right => widgets.binary_search_by_key(&coord.0, |w| w.rect().pos.0),
            Direction::Down => widgets.binary_search_by_key(&coord.1, |w| w.rect().pos.1),
            Direction::Left => widgets.binary_search_by(|w| w.rect().pos.0.cmp(&coord.0).reverse()),
            Direction::Up => widgets.binary_search_by(|w| w.rect().pos.1.cmp(&coord.1).reverse()),
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
                if self.direction.is_reversed() {
                    if i == widgets.len() || !widgets[i].rect().contains(coord) {
                        return None;
                    }
                    i
                } else {
                    if i == 0 || !widgets[i - 1].rect().contains(coord) {
                        return None;
                    }
                    i - 1
                }
            }
        };
        Some(&widgets[index])
    }

    /// Call `f` on each child intersecting the given `rect`
    pub fn for_children<W: Widget, F: FnMut(&W)>(self, widgets: &[W], rect: Rect, mut f: F) {
        let (pos, end) = match self.direction.is_reversed() {
            false => (rect.pos, rect.pos + rect.size),
            true => (rect.pos + rect.size, rect.pos),
        };
        let start = match self.binary_search(widgets, pos) {
            Ok(i) => i,
            Err(i) if i > 0 => {
                let j = i - 1;
                if widgets[j].rect().contains(pos) {
                    j
                } else {
                    i
                }
            }
            Err(_) => 0,
        };

        for i in start..widgets.len() {
            let child = &widgets[i];
            let do_break = match self.direction.as_direction() {
                Direction::Right => child.rect().pos.0 >= end.0,
                Direction::Down => child.rect().pos.1 >= end.1,
                Direction::Left => child.rect().pos_end().0 < end.0,
                Direction::Up => child.rect().pos_end().1 < end.1,
            };
            if do_break {
                break;
            }
            f(child);
        }
    }
}
