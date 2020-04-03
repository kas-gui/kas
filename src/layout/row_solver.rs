// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;

use super::{AxisInfo, RowStorage, RowTemp, RulesSetter, RulesSolver, SizeRules};
use crate::geom::{Coord, Rect};
use crate::{Direction, Directional, Widget};

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
            // TODO: cache this for use by set_rect?
            let (rules, widths) = storage.rules_and_widths();
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
    crect: Rect,
    offsets: T,
    direction: D,
    _s: PhantomData<S>,
}

impl<D: Directional, T: RowTemp, S: RowStorage> RowSetter<D, T, S> {
    pub fn new(rect: Rect, (dir, len): (D, usize), storage: &mut S) -> Self {
        let mut offsets = T::default();
        offsets.set_len(len);
        storage.set_dim(len);

        let (pos, width) = match dir.is_horizontal() {
            true => (rect.pos.0, rect.size.0),
            false => (rect.pos.1, rect.size.1),
        };

        if len > 0 {
            let (rules, widths) = storage.rules_and_widths();
            SizeRules::solve_seq(widths, rules, width);
            if dir.is_reversed() {
                offsets.as_mut()[len - 1] = pos as u32;
                for i in (0..(len - 1)).rev() {
                    let i1 = i + 1;
                    let m1 = storage.rules()[i1].margins().1;
                    let m0 = storage.rules()[i].margins().0;
                    offsets.as_mut()[i] =
                        offsets.as_mut()[i1] + storage.widths()[i1] + m1.max(m0) as u32;
                }
            } else {
                offsets.as_mut()[0] = pos as u32;
                for i in 1..len {
                    let i1 = i - 1;
                    let m1 = storage.rules()[i1].margins().1;
                    let m0 = storage.rules()[i].margins().0;
                    offsets.as_mut()[i] =
                        offsets.as_mut()[i1] + storage.widths()[i1] + m1.max(m0) as u32;
                }
            }
        }

        RowSetter {
            crect: rect,
            offsets,
            direction: dir,
            _s: Default::default(),
        }
    }
}

impl<D: Directional, T: RowTemp, S: RowStorage> RulesSetter for RowSetter<D, T, S> {
    type Storage = S;
    type ChildInfo = usize;

    fn child_rect(&mut self, storage: &mut Self::Storage, index: Self::ChildInfo) -> Rect {
        if self.direction.is_horizontal() {
            self.crect.pos.0 = self.offsets.as_mut()[index] as i32;
            self.crect.size.0 = storage.widths()[index];
        } else {
            self.crect.pos.1 = self.offsets.as_mut()[index] as i32;
            self.crect.size.1 = storage.widths()[index];
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
