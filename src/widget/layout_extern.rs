//! Layout hooks for using an external layout engine


use std::fmt;

use crate::widget::Core;
use crate::toolkit::TkWidget;

/// How child widgets are arranged
pub enum ChildLayout {
    /// Implies no more than one child widget.
    None,
    /// Child widgets are arranged in a horizontol row, left to right.
    Horizontal,
    /// Child widgets are arranged in a vertical column, top to bottom.
    Vertical,
    /// Child widgets are arranged in a grid.
    Grid,
}

/// Column and row location information, `(col, row, col-span, row-span)`.
/// 
/// Column and row `0, 0` is the top-left position. Spans are usually 1, but
/// may be larger; in this case columns from `col` to `col + col-span - 1` are
/// occupied.
pub type GridPos = (i32, i32, i32, i32);

/// Size and position handling for widgets, the universal interface to the
/// layout system.
/// 
/// Note that this trait has very different internals depending on which layout
/// engine is used.
// TODO: move Cassowary methods to a sub-trait if we get multi-trait object support
pub trait Layout: Core + fmt::Debug {
    /// Layout for child widgets
    fn child_layout(&self) -> ChildLayout;
    
    /// Per child positioning for grid layout
    /// 
    /// This returns `None` if `index` is out of range or if no position
    /// information was supplied for this widget. Toolkits should gracefully
    /// handle missing position information.
    fn grid_pos(&self, index: usize) -> Option<GridPos>;

    /// Read position and size of widget from the toolkit
    /// 
    /// This is for use when the toolkit does layout adjustment of widgets.
    fn sync_size(&mut self, tk: &TkWidget);
}
