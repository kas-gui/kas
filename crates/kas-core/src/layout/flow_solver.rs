// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Flow solver

use super::{AxisInfo, SizeRules};
use super::{RulesSetter, RulesSolver};
use crate::dir::{Direction, Directional};
use crate::geom::{Coord, Rect, Size};

/// Storage required by [`FlowSolver`] and [`FlowSetter`]
#[derive(Clone, Debug, Default)]
pub struct FlowStorage {
    rules: Vec<SizeRules>,
    /// The length should be `self.breaks.len() + 1`.
    height_rules: Vec<SizeRules>,
    widths: Vec<i32>,
    /// These are the indices at which a new line starts. It may be assumed that
    /// a new line starts at index zero, hence 0 is not present in this list.
    breaks: Vec<usize>,
}

/// A [`RulesSolver`] for flows
///
/// The flow direction is currently restricted to lines which flow to the right,
/// wrapping down (as in English text).
///
/// Margins of the "flow" as a whole are the maximum of all item margins since
/// it is not known in advance which items will be on the first/last line or at
/// the start/end of each line.
pub struct FlowSolver {
    axis: AxisInfo,
    direction: Direction,
    secondary_is_reversed: bool,
    opt_rules: Option<SizeRules>,
    rules: SizeRules,
}

impl FlowSolver {
    /// Construct a solver
    ///
    /// Parameters:
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `direction`: primary direction of flow (lines)
    /// - `secondary_is_reversed`: true if the direction in which lines wrap is
    ///     left or up (this corresponds to [`Directional::is_reversed`])
    /// - `len`:  and total number of items
    /// - `storage`: reference to persistent storage
    pub fn new(
        axis: AxisInfo,
        direction: Direction,
        secondary_is_reversed: bool,
        len: usize,
        storage: &mut FlowStorage,
    ) -> Self {
        storage.rules.resize(len, SizeRules::EMPTY);
        storage.widths.resize(len, 0);

        if direction.is_horizontal() || axis.is_horizontal() {
            storage.breaks.clear();
            storage.height_rules.clear();
        }

        // If the flow consists of rows, then we solve widths on the vertical
        // axis. For columns we can't do anything useful here.
        if axis.has_fixed && direction.is_horizontal() && len != 0 {
            debug_assert!(axis.is_vertical());
            // Assume we already have rules for the other axis; solve for the given width
            let width = axis.other_axis;

            let mut total = storage.rules[0];
            let mut start = 0;
            for i in 1..storage.rules.len() {
                let sum = total.appended(storage.rules[i]);
                if sum.min_size() <= width {
                    total = sum;
                    continue;
                }

                // Line break. Solve widths for the previous line:
                SizeRules::solve_widths_with_total(
                    &mut storage.widths[start..i],
                    &mut storage.rules[start..i],
                    total,
                    width,
                );
                storage.breaks.push(i);
                start = i;
                total = storage.rules[i];
            }

            // Final line:
            SizeRules::solve_widths_with_total(
                &mut storage.widths[start..],
                &mut storage.rules[start..],
                total,
                width,
            );
        }

        storage.height_rules.reserve_exact(storage.breaks.len() + 1);

        FlowSolver {
            axis,
            direction,
            secondary_is_reversed,
            opt_rules: None,
            rules: SizeRules::EMPTY,
        }
    }

    /// Set column width
    ///
    /// When the primary direction is vertical, the column width cannot be
    /// inferred. Set it with this method. (In other cases this does nothing.)
    ///
    /// This does not directly affect the returned [`SizeRules`], but it *does*
    /// affect the width supplied to children when inferring their height
    /// (see [`AxisInfo::other`]), which could be useful if e.g. child widgtes
    /// contain text which wraps at this width.
    ///
    /// Further note: the result of [`Self::finish`] for the horizontal axis
    /// will be just the maximum [`SizeRules`] of all children. You may wish to
    /// call [`SizeRules::multiply_with_margin`] for the horizontal axis to
    /// reserve enough room for multiple columns.
    pub fn set_column_properties(&mut self, width: i32) {
        if self.direction.is_vertical() && self.axis.is_vertical() {
            self.axis.map_other(|w| w.min(width));
        }
    }
}

impl RulesSolver for FlowSolver {
    type Storage = FlowStorage;
    type ChildInfo = usize;

    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        storage: &mut Self::Storage,
        index: Self::ChildInfo,
        child_rules: CR,
    ) {
        if self.direction.is_horizontal() && self.axis.has_fixed {
            // For rows, use per-item widths (solved in Self::new)
            self.axis.other_axis = storage.widths[index];
        }
        let child_rules = child_rules(self.axis);
        if self.direction.is_horizontal() == self.axis.is_horizontal() {
            storage.rules[index] = child_rules;
        }

        if self.direction.is_horizontal() == self.axis.is_horizontal() {
            // We calculate the ideal size by appending all items into one line:
            self.opt_rules = Some(if let Some(rules) = self.opt_rules {
                if self.direction.is_reversed() {
                    child_rules.appended(rules)
                } else {
                    rules.appended(child_rules)
                }
            } else {
                child_rules
            });
        } else {
            if storage.breaks.contains(&index) {
                storage.height_rules.push(self.rules);

                self.opt_rules = Some(if let Some(rules) = self.opt_rules {
                    if self.secondary_is_reversed {
                        rules.appended(self.rules)
                    } else {
                        self.rules.appended(rules)
                    }
                } else {
                    self.rules
                });
                self.rules = SizeRules::EMPTY;
            }
        }

        self.rules = self.rules.max(child_rules);
    }

    fn finish(self, storage: &mut Self::Storage) -> SizeRules {
        if self.direction.is_horizontal() == self.axis.is_horizontal() {
            let min = self.rules.min_size();
            let ideal = self.opt_rules.unwrap_or(SizeRules::EMPTY).ideal_size();
            let stretch = self.rules.stretch();
            SizeRules::new(min, ideal, stretch).with_margins(self.rules.margins())
        } else {
            let rules = if let Some(rules) = self.opt_rules {
                if self.secondary_is_reversed {
                    rules.appended(self.rules)
                } else {
                    self.rules.appended(rules)
                }
            } else {
                self.rules
            };

            storage.height_rules.push(rules);
            debug_assert_eq!(storage.breaks.len() + 1, storage.height_rules.len());
            rules
        }
    }
}

/// A [`RulesSetter`] for flows
pub struct FlowSetter {
    pos: Coord,
    offsets: Vec<i32>,
    direction: Direction,
    secondary_is_reversed: bool,
    heights: Vec<i32>, // by row
    row_offsets: Vec<i32>,
}

impl FlowSetter {
    /// Construct a setter
    ///
    /// Parameters:
    ///
    /// -   `rect`: the [`Rect`] within which to position children
    /// - `direction`: primary direction of flow (lines)
    /// - `secondary_is_reversed`: true if the direction in which lines wrap is
    ///     left or up (this corresponds to [`Directional::is_reversed`])
    /// - `len`:  and total number of items
    /// - `storage`: reference to persistent storage
    pub fn new(
        rect: Rect,
        direction: Direction,
        secondary_is_reversed: bool,
        len: usize,
        storage: &mut FlowStorage,
    ) -> Self {
        let offsets = vec![0; len];
        assert_eq!(storage.rules.len(), len);
        let mut heights = vec![];

        if direction.is_vertical() {
            // TODO: solve storage.breaks (wrap points) here, then solve storage.widths (which are
            // heights in this case) and finally column widths (which requires per-item width rules
            // or just fixed column widths).
            todo!()
        }

        if len != 0 {
            let height = rect.size.extract(direction.flipped());
            heights = vec![0; storage.height_rules.len()];
            SizeRules::solve_widths(&mut heights, &storage.height_rules, height);
        }

        let mut row = FlowSetter {
            pos: rect.pos,
            offsets,
            direction,
            secondary_is_reversed,
            heights,
            row_offsets: vec![],
        };
        row.update_offsets(storage);
        row
    }

    fn update_offsets(&mut self, storage: &mut FlowStorage) {
        fn set_offsets(
            pos: i32,
            rules: &[SizeRules],
            sizes: &[i32],
            offsets: &mut [i32],
            iter: impl ExactSizeIterator<Item = usize>,
            is_break: impl Fn(usize) -> bool,
        ) {
            debug_assert_eq!(rules.len(), sizes.len());
            debug_assert_eq!(rules.len(), iter.len());

            let mut caret = pos;
            let mut margin = 0;
            for i in iter {
                let margins = rules[i].margins_i32();
                if is_break(i) {
                    caret = pos;
                } else {
                    caret += margin.max(margins.0);
                }
                margin = margins.1;

                offsets[i] = caret;
                caret += sizes[i];
            }
        }

        let len = self.offsets.len();
        if len == 0 {
            return;
        }

        let pos = self.pos.extract(self.direction);
        if !self.direction.is_reversed() {
            set_offsets(
                pos,
                &storage.rules,
                &storage.widths,
                &mut self.offsets,
                0..len,
                |i| i == 0 || storage.breaks.contains(&i),
            );
        } else {
            set_offsets(
                pos,
                &storage.rules,
                &storage.widths,
                &mut self.offsets,
                (0..len).rev(),
                |i| i == len - 1 || storage.breaks.contains(&(i + 1)),
            );
        }

        let pos = self.pos.extract(self.direction.flipped());
        let len = storage.height_rules.len();
        self.row_offsets.resize(len, 0);
        let offsets = &mut self.row_offsets;
        if !self.secondary_is_reversed {
            set_offsets(
                pos,
                &mut storage.height_rules,
                &self.heights,
                offsets,
                0..len,
                |_| false,
            );
        } else {
            set_offsets(
                pos,
                &mut storage.height_rules,
                &self.heights,
                offsets,
                (0..len).rev(),
                |_| false,
            );
        }
    }
}

impl RulesSetter for FlowSetter {
    type Storage = FlowStorage;
    type ChildInfo = usize;

    fn child_rect(&mut self, storage: &mut Self::Storage, index: Self::ChildInfo) -> Rect {
        let row = match storage.breaks.binary_search(&index) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        let w = storage.widths[index];
        let h = self.heights[row];
        let x = self.offsets[index];
        let y = self.row_offsets[row];

        if self.direction.is_horizontal() {
            Rect::new(Coord(x, y), Size(w, h))
        } else {
            Rect::new(Coord(y, x), Size(h, w))
        }
    }
}
