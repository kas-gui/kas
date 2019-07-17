// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout hooks for using an external layout engine


use std::fmt;

use crate::widget::Core;
use crate::toolkit::TkWidget;

#[doc(hidden)]
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

#[doc(hidden)]
/// Column and row location information, `(col, row, col-span, row-span)`.
/// 
/// Column and row `0, 0` is the top-left position. Spans are usually 1, but
/// may be larger; in this case columns from `col` to `col + col-span - 1` are
/// occupied.
pub type GridPos = (i32, i32, i32, i32);

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
/// 
/// [`Widget`]: kas::Widget
*/
// TODO: move Cassowary methods to a sub-trait if we get multi-trait object support
pub trait Layout: Core + fmt::Debug {
    #[doc(hidden)]
    /// Layout for child widgets
    fn child_layout(&self) -> ChildLayout;
    
    #[doc(hidden)]
    /// Per child positioning for grid layout
    /// 
    /// This returns `None` if `index` is out of range or if no position
    /// information was supplied for this widget. If `None` is returned, then
    /// the first cell (`GridPos(0,0,1,1)`) should be assumed.
    fn grid_pos(&self, index: usize) -> Option<GridPos>;

    #[doc(hidden)]
    /// Read position and size of widget from the toolkit
    /// 
    /// This is for use when the toolkit does layout adjustment of widgets.
    fn sync_size(&mut self, tk: &dyn TkWidget);
}
