// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver

use log::trace;
use std::fmt;

use super::{AxisInfo, Margins, SizeRules};
use crate::draw::SizeHandle;
use crate::geom::{Coord, Rect, Size};
use crate::{AlignHints, WidgetConfig};

/// A [`SizeRules`] solver for layouts
///
/// Typically, a solver is invoked twice, once for each axis, before the
/// corresponding [`RulesSetter`] is invoked. This is managed by [`solve`].
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

    /// Called once for each child. For most layouts the order is important.
    fn child_rect(&mut self, storage: &mut Self::Storage, child_info: Self::ChildInfo) -> Rect;
}

/// Cache used by [`solve`] and [`solve_and_set`]
pub struct SolveCache {
    // Technically we don't need to store min and ideal here, but it simplifies
    // the API for very little real cost.
    min: Size,
    ideal: Size,
    margins: Margins,
    refresh_rules: bool,
    last_width: u32,
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
    pub fn new(widget: &mut dyn WidgetConfig, size_handle: &mut dyn SizeHandle) -> Self {
        let w = widget.size_rules(size_handle, AxisInfo::new(false, None));
        let h = widget.size_rules(size_handle, AxisInfo::new(true, Some(w.ideal_size())));

        let min = Size(w.min_size(), h.min_size());
        let ideal = Size(w.ideal_size(), h.ideal_size());
        let margins = Margins::hv(w.margins(), h.margins());
        trace!(
            "layout::solve: min={:?}, ideal={:?}, margins={:?}",
            min,
            ideal,
            margins
        );
        let refresh_rules = false;
        let last_width = ideal.0;
        SolveCache {
            min,
            ideal,
            margins,
            refresh_rules,
            last_width,
        }
    }

    /// Force updating of size rules
    ///
    /// This should be called whenever widget size rules have been changed. It
    /// forces [`SolveCache::apply_rect`] to recompute these rules when next
    /// called.
    pub fn invalidate_rule_cache(&mut self) {
        self.refresh_rules = true;
    }

    /// Apply layout solution to a widget
    ///
    /// The widget's layout is solved for the given `rect` and assigned.
    /// If `inner_margin` is true, margins are internal to this `rect`; if not,
    /// the caller is responsible for handling margins.
    ///
    /// If [`SolveCache::invalidate_rule_cache`] was called since rules were
    /// last calculated then this method will recalculate all rules; otherwise
    /// it will only do so if necessary (when dimensions do not match those
    /// last used).
    pub fn apply_rect(
        &mut self,
        widget: &mut dyn WidgetConfig,
        size_handle: &mut dyn SizeHandle,
        mut rect: Rect,
        inner_margin: bool,
    ) {
        // We call size_rules not because we want the result, but because our
        // spec requires that we do so before calling set_rect.
        if self.refresh_rules {
            let w = widget.size_rules(size_handle, AxisInfo::new(false, None));
            self.min.0 = w.min_size();
            self.ideal.0 = w.ideal_size();
            self.margins.horiz = w.margins();
        }
        let mut width = rect.size.0;
        if inner_margin {
            width -= (self.margins.horiz.0 + self.margins.horiz.1) as u32;
        }

        if self.refresh_rules || width != self.last_width {
            let h = widget.size_rules(size_handle, AxisInfo::new(true, Some(width)));
            self.min.1 = h.min_size();
            self.ideal.1 = h.ideal_size();
            self.margins.vert = h.margins();
            self.last_width = width;
        }

        if inner_margin {
            rect.pos += Coord(self.margins.horiz.0 as i32, self.margins.vert.0 as i32);
            rect.size.0 = width;
            rect.size.1 -= (self.margins.vert.0 + self.margins.vert.1) as u32;
        }
        widget.set_rect(rect, AlignHints::NONE);

        trace!(
            "layout::solve_and_set for size={:?} has hierarchy:{}",
            rect.size,
            WidgetHeirarchy(widget, 0),
        );

        self.refresh_rules = false;
    }
}

struct WidgetHeirarchy<'a>(&'a dyn WidgetConfig, usize);
impl<'a> fmt::Display for WidgetHeirarchy<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "\n{}{}\t{}\tpos={:?}\tsize={:?}",
            "- ".repeat(self.1),
            self.0.id(),
            self.0.widget_name(),
            self.0.rect().pos,
            self.0.rect().size,
        )?;

        for i in 0..self.0.len() {
            WidgetHeirarchy(self.0.get(i).unwrap(), self.1 + 1).fmt(f)?;
        }
        Ok(())
    }
}
