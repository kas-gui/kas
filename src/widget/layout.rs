// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout code using the Cassowary constraint solver

use std::fmt;

use crate::cw;
use crate::toolkit::TkWidget;
use crate::widget::Coord;
use crate::widget::Core;

/// An internal detail.
///
/// This trait is used internally and by toolkits. Users should not use it
/// directly, in part because it may have a very different body depending on
/// feature flags.
/*
/// Size and position handling for widgets, the universal interface to the
/// layout system.
///
/// This is a base trait of [`Widget`] and should not need to be used directly.
/// It is implemented automatically by the `derive(Widget)` macro.
///
/// Note that this trait has very different internals depending on which layout
/// engine is used.
*/
pub trait Layout: Core + fmt::Debug {
    #[doc(hidden)]
    /// Initialise the constraint solver.
    ///
    /// This function applies constraints to the solver based on the current
    /// widget's size requirements. Once the constraint solver has found a
    /// solution, `apply_constraints` may be called to update the widget layout.
    ///
    /// If `use_default` is true, then this widget's preferred size is used as
    /// the initial value, otherwise it's current size is used.
    ///
    /// The default implementation may suffice for simple widgets without
    /// children, but must be overriden by any parent widget.
    // TODO: because of width-for-height relations it may be necessary to
    // adjust this, e.g. solve for width first then for height.
    fn init_constraints(&self, tk: &dyn TkWidget, s: &mut cw::Solver, use_default: bool);

    #[doc(hidden)]
    /// Apply constraints from the solver.
    ///
    /// See the `init_constraints` documentation.
    ///
    /// `pos` is the widget's position relative to the parent window.
    fn apply_constraints(&mut self, tk: &mut dyn TkWidget, s: &cw::Solver, pos: Coord);
}

#[macro_export]
macro_rules! cw_var {
    ($w:expr, w) => {
        $crate::cw::Variable::from_usize($w.number() as usize)
    };
    ($w:expr, h) => {
        $crate::cw::Variable::from_usize($w.number() as usize + 0x1000_0000)
    };
    ($w:expr, w, $n:expr) => {
        $crate::cw::Variable::from_usize($w.number() as usize + (0x1_0000 * ($n as usize + 1)))
    };
    ($w:expr, h, $n:expr) => {
        $crate::cw::Variable::from_usize(
            $w.number() as usize + (0x1_0000 * ($n as usize + 1)) + 0x1000_0000,
        )
    };
}
