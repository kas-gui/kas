//! Layout code using the Cassowary constraint solver

use std::fmt;

use crate::cw;
use crate::widget::Coord;
use crate::widget::Core;
use crate::toolkit::TkWidget;

/// Size and position handling for widgets, the universal interface to the
/// layout system.
/// 
/// Note that this trait has very different internals depending on which layout
/// engine is used.
pub trait Layout: Core + fmt::Debug {
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
    #[cfg(feature = "cassowary")]
    fn init_constraints(&self, tk: &TkWidget,
        s: &mut cw::Solver, use_default: bool);
    
    /// Apply constraints from the solver.
    /// 
    /// See the `init_constraints` documentation.
    /// 
    /// `pos` is the widget's position relative to the parent window.
    #[cfg(feature = "cassowary")]
    fn apply_constraints(&mut self, tk: &TkWidget, s: &cw::Solver, pos: Coord);
}

#[macro_export]
macro_rules! cw_var {
    ($w:expr, w) => { $crate::cw::Variable::from_usize($w.number() as usize) };
    ($w:expr, h) => { $crate::cw::Variable::from_usize(($w.number() + 0x1000_0000) as usize) };
}
