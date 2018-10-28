//! Layout hooks for using an external layout engine


use std::fmt;

use crate::widget::WidgetCore;
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
pub trait Layout: WidgetCore + fmt::Debug {
    /// Layout for child widgets
    fn child_layout(&self) -> ChildLayout;
    
    /// Per child positioning for grid layout
    /// 
    /// This may only be called if `self.child_layout() == ChildLayout::Grid`.
    /// The widget number must be less than `self.len()`.
    /// 
    /// Return value: `(col, row, col-span, row-span)` where `col` and `row`
    /// start at 0 (top-left position).
    fn grid_pos(&self, index: usize) -> Option<GridPos>;

    /// Read position and size of widget from the toolkit
    /// 
    /// This is for use when the toolkit does layout adjustment of widgets.
    fn sync_size(&mut self, tk: &TkWidget);
}


/// Implements `Layout` for widgets with no children
#[macro_export]
macro_rules! impl_layout_simple {
    // this evil monstrosity matches <A, B: T, C: S+T>
    // but because there is no "zero or one" rule, also <D: S: T>
    ($ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )* ),* >) => {
        impl< $( $N $(: $b0 $(+$b)* )* ),* >
            $crate::widget::Layout
            for $ty< $( $N ),* >
        {
            fn child_layout(&self) -> $crate::widget::ChildLayout {
                $crate::widget::ChildLayout::None
            }
            
            fn grid_pos(&self, _index: usize) -> Option<$crate::widget::GridPos> {
                None
            }

            fn sync_size(&mut self, tk: &$crate::toolkit::TkWidget) {
                let new_rect = tk.get_rect(self.get_tkd());
                *self.rect_mut() = new_rect;
            }
        }
    };
    ($ty:ident) => {
        $crate::impl_layout_simple!($ty<>);
    };
}

/// Implements `Layout` for widgets with a single child, with specified name
#[macro_export]
macro_rules! impl_layout_single {
    // this evil monstrosity matches <A, B: T, C: S+T>
    // but because there is no "zero or one" rule, also <D: S: T>
    ($ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )* ),* >, $child:ident) => {
        impl< $( $N $(: $b0 $(+$b)* )* ),* >
            $crate::widget::Layout
            for $ty< $( $N ),* >
        {
            fn child_layout(&self) -> $crate::widget::ChildLayout {
                $crate::widget::ChildLayout::None
            }
            
            fn grid_pos(&self, _index: usize) -> Option<$crate::widget::GridPos> {
                None
            }

            fn sync_size(&mut self, tk: &$crate::toolkit::TkWidget) {
                let new_rect = tk.get_rect(self.get_tkd());
                *self.rect_mut() = new_rect;
                
                self.$child.sync_size(tk)
            }
        }
    };
    ($ty:ident, $child:ident) => {
        $crate::impl_layout_single!($ty<>, $child);
    };
}

#[macro_export]
macro_rules! select_child_layout {
    (single) => { $crate::widget::ChildLayout::None };
    (horizontal) => { $crate::widget::ChildLayout::Horizontal };
    (vertical) => { $crate::widget::ChildLayout::Vertical };
    (grid) => { $crate::widget::ChildLayout::Grid };
}

#[macro_export]
macro_rules! impl_grid_pos_item {
    ($n:expr, $index:ident; ) => {
        // missing information; return None
    };
    ($n:expr, $index:ident; [$c:expr, $r:expr]) => {
        if $index == $n {
            return Some(($c, $r, 1, 1));
        }
    };
    ($n:expr, $index:ident; [$c:expr, $r:expr, $cs:expr, $rs:expr]) => {
        if $index == $n {
            return Some(($c, $r, $cs, $rs));
        }
    }
}

#[macro_export]
macro_rules! impl_grid_pos {
    ($n:expr, $index:ident; ) => ();
    ($n:expr, $index:ident; item $([ $($pos:expr),* ])*) => {
        $crate::impl_grid_pos_item!($n, $index; $([ $($pos),* ])*)
    };
    ($n:expr, $index:ident; item $([ $($pos:expr),* ])*, $(item $([ $($xpos:expr),* ])* ),*) => {
        $crate::impl_grid_pos_item!($n, $index; $([ $($pos),* ])*);
        $crate::impl_grid_pos!($n + 1, $index; $(item $([ $($xpos),* ])* ),*)
    };
}

/// Implements `Layout`
#[macro_export]
macro_rules! impl_widget_layout {
    // this evil monstrosity matches <A, B: T, C: S+T>
    // but because there is no "zero or one" rule, also <D: S: T>
    ($ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )* ),* >;
        $direction:ident;
        $($([$($pos:expr),*])* $name:ident),*) =>
    {
        impl< $( $N $(: $b0 $(+$b)* )* ),* >
            $crate::widget::Layout
            for $ty< $( $N ),* >
        {
            fn child_layout(&self) -> $crate::widget::ChildLayout {
                $crate::select_child_layout!($direction)
            }
            
            fn grid_pos(&self, _index: usize) -> Option<$crate::widget::GridPos> {
//                 trace_macros!(true);
                $crate::impl_grid_pos!(0, _index; $(item $([ $($pos),* ])* ),*);
//                 trace_macros!(false);
                None
            }

            fn sync_size(&mut self, tk: &$crate::toolkit::TkWidget) {
                let new_rect = tk.get_rect(self.get_tkd());
                *self.rect_mut() = new_rect;
                
                $(self.$name.sync_size(tk);)*
            }
        }
    };
    ($ty:ident; $direction:ident; $($name:ident),*) => {
        $crate::impl_widget_layout!($ty<>; $direction; $($name),*);
    };
}
