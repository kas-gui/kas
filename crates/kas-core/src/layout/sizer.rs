// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver

use super::{AxisInfo, Margins, SizeRules};
use crate::cast::Conv;
use crate::geom::{Rect, Size};
use crate::layout::AlignHints;
use crate::theme::SizeCx;
use crate::util::WidgetHierarchy;
use crate::{Node, Tile};
use log::trace;

/// A [`SizeRules`] solver for layouts
///
/// Typically, a solver is invoked twice, once for each axis, before the
/// corresponding [`RulesSetter`] is invoked. This is managed by [`SolveCache`].
///
/// Implementations require access to storage able to persist between multiple
/// solver runs and a subsequent setter run. This storage is of type
/// [`RulesSolver::Storage`] and is passed via reference to the constructor.
pub trait RulesSolver {
    /// Type of storage
    type Storage: Clone;

    /// Type required by [`RulesSolver::for_child`] (see implementation documentation)
    type ChildInfo;

    /// Called once for each child. For most layouts the order is important.
    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        storage: &mut Self::Storage,
        child_info: Self::ChildInfo,
        child_rules: CR,
    );

    /// Called at the end to output [`SizeRules`].
    ///
    /// Note that this does not include margins!
    fn finish(self, storage: &mut Self::Storage) -> SizeRules;
}

/// Resolves a [`RulesSolver`] solution for each child
pub trait RulesSetter {
    /// Type of storage
    type Storage: Clone;

    /// Type required by [`RulesSolver::for_child`] (see implementation documentation)
    type ChildInfo;

    /// Called once for each child. The order is unimportant.
    fn child_rect(&mut self, storage: &mut Self::Storage, child_info: Self::ChildInfo) -> Rect;

    /// Calculates the maximal rect of a given child
    ///
    /// This assumes that all other entries have minimum size.
    fn maximal_rect_of(&mut self, storage: &mut Self::Storage, index: Self::ChildInfo) -> Rect;
}

/// Solve size rules for a widget
///
/// Automatic layout solving requires that a widget's `size_rules` method is
/// called for each axis before `set_rect`. This method simply calls
/// `size_rules` on each axis.
///
/// If `size_rules` is not called, internal layout may be poor (depending on the
/// widget). If widget content changes, it is recommended to call
/// `solve_size_rules` and `set_rect` again.
///
/// Parameters `x_size` and `y_size` should be passed where this dimension is
/// fixed and are used e.g. for text wrapping.
pub fn solve_size_rules<W: Tile + ?Sized>(
    widget: &mut W,
    cx: &mut SizeCx,
    x_size: Option<i32>,
    y_size: Option<i32>,
) {
    trace!(
        "solve_size_rules({}, _, {:?}, {:?})",
        widget.identify(),
        x_size,
        y_size
    );
    widget.size_rules(cx, AxisInfo::new(false, y_size));
    widget.size_rules(cx, AxisInfo::new(true, x_size));
}

/// Size solver
///
/// This struct is used to solve widget layout, read size constraints and
/// cache the results until the next solver run.
///
/// [`SolveCache::find_constraints`] constructs an instance of this struct,
/// solving for size constraints.
///
/// [`SolveCache::apply_rect`] accepts a [`Rect`], updates constraints as
/// necessary and sets widget positions within this `rect`.
#[derive(Default)]
pub struct SolveCache {
    // Technically we don't need to store min and ideal here, but it simplifies
    // the API for very little real cost.
    min: Size,
    ideal: Size,
    margins: Margins,
    last_width: i32,
}

impl SolveCache {
    /// Get the minimum size
    ///
    /// If `inner_margin` is true, margins are included in the result.
    pub fn min(&self, inner_margin: bool) -> Size {
        if inner_margin {
            self.margins.pad(self.min)
        } else {
            self.min
        }
    }

    /// Get the ideal size
    ///
    /// If `inner_margin` is true, margins are included in the result.
    pub fn ideal(&self, inner_margin: bool) -> Size {
        if inner_margin {
            self.margins.pad(self.ideal)
        } else {
            self.ideal
        }
    }

    /// Get the margins
    pub fn margins(&self) -> Margins {
        self.margins
    }

    /// Calculate required size of widget
    ///
    /// Assumes no explicit alignment.
    pub fn find_constraints(&mut self, mut widget: Node<'_>, cx: &mut SizeCx) {
        let start = std::time::Instant::now();

        let w = widget.size_rules(cx, AxisInfo::new(false, None));
        let h = widget.size_rules(cx, AxisInfo::new(true, Some(w.ideal_size())));

        self.min = Size(w.min_size(), h.min_size());
        self.ideal = Size(w.ideal_size(), h.ideal_size());
        self.margins = Margins::hv(w.margins(), h.margins());

        log::trace!(
            target: "kas_perf::layout", "find_constraints: {}μs",
            start.elapsed().as_micros(),
        );
        log::debug!(
            "find_constraints: min={:?}, ideal={:?}, margins={:?}",
            &self.min,
            &self.ideal,
            &self.margins
        );
        self.last_width = self.ideal.0;
    }

    /// Apply layout solution to a widget
    ///
    /// The widget's layout is solved for the given `rect` and assigned.
    /// If `inner_margin` is true, margins are internal to this `rect`; if not,
    /// the caller is responsible for handling margins.
    ///
    /// This method will only recalculate rules as necessary (namely height
    /// rules when the width has changed).
    pub fn apply_rect(
        &mut self,
        mut widget: Node<'_>,
        cx: &mut SizeCx,
        mut rect: Rect,
        inner_margin: bool,
    ) {
        let start = std::time::Instant::now();

        let mut width = rect.size.0;
        if inner_margin {
            width -= self.margins.sum_horiz();
        }

        // We call size_rules not because we want the result, but to allow
        // internal layout solving.
        if width != self.last_width {
            let h = widget.size_rules(cx, AxisInfo::new(true, Some(width)));
            self.min.1 = h.min_size();
            self.ideal.1 = h.ideal_size();
            self.margins.vert = h.margins();
            self.last_width = width;
        }

        if inner_margin {
            rect.pos += Size::conv((self.margins.horiz.0, self.margins.vert.0));
            rect.size.0 = width;
            rect.size.1 -= self.margins.sum_vert();
        }
        widget.set_rect(cx, rect, AlignHints::NONE);

        log::trace!(target: "kas_perf::layout", "apply_rect: {}μs", start.elapsed().as_micros());
    }

    /// Print widget heirarchy in the trace log
    ///
    /// This is sometimes called after [`Self::apply_rect`].
    pub fn print_widget_heirarchy(&mut self, widget: &dyn Tile) {
        let rect = widget.rect();
        let hier = WidgetHierarchy::new(widget, None);
        log::trace!(
            target: "kas_core::layout::hierarchy",
            "apply_rect: rect={rect:?}:{hier}",
        );
    }
}
