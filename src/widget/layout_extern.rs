//! Layout hooks for using an external layout engine


use std::fmt;

use crate::widget::WidgetCore;
use crate::toolkit::Toolkit;

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

/// Size and position handling for widgets, the universal interface to the
/// layout system.
/// 
/// Note that this trait has very different internals depending on which layout
/// engine is used.
// TODO: move Cassowary methods to a sub-trait if we get multi-trait object support
pub trait Layout: WidgetCore + fmt::Debug {
    /// Layout for child widgets
    fn child_layout(&self) -> ChildLayout;

    /// Read position and size of widget from the toolkit
    /// 
    /// This is for use when the toolkit does layout adjustment of widgets.
    fn sync_size(&mut self, tk: &Toolkit);
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

            fn sync_size(&mut self, tk: &$crate::toolkit::Toolkit) {
                let new_rect = tk.tk_widget().get_rect(self.get_tkd());
                *self.rect_mut() = new_rect;
            }
        }
    };
    ($ty:ident) => {
        impl_layout_simple!($ty<>);
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

            fn sync_size(&mut self, tk: &$crate::toolkit::Toolkit) {
                let new_rect = tk.tk_widget().get_rect(self.get_tkd());
                *self.rect_mut() = new_rect;
                
                self.$child.sync_size(tk)
            }
        }
    };
    ($ty:ident, $child:ident) => {
        impl_layout_single!($ty<>, $child);
    };
}

#[macro_export]
macro_rules! select_child_layout {
    (single) => { $crate::widget::ChildLayout::None };
    (horizontal) => { $crate::widget::ChildLayout::Horizontal };
    (vertical) => { $crate::widget::ChildLayout::Vertical };
    (grid) => { $crate::widget::ChildLayout::Grid };
}

/// Implements `Layout`
#[macro_export]
macro_rules! impl_widget_layout {
    // this evil monstrosity matches <A, B: T, C: S+T>
    // but because there is no "zero or one" rule, also <D: S: T>
    ($ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )* ),* >;
        $direction:ident;
        $($wname:ident),*) =>
    {
        impl< $( $N $(: $b0 $(+$b)* )* ),* >
            $crate::widget::Layout
            for $ty< $( $N ),* >
        {
            fn child_layout(&self) -> $crate::widget::ChildLayout {
                select_child_layout!($direction)
            }

            fn sync_size(&mut self, tk: &Toolkit) {
                let new_rect = tk.tk_widget().get_rect(self.get_tkd());
                *self.rect_mut() = new_rect;
                
                $(self.$wname.sync_size(tk);)*
            }
        }
    };
    ($ty:ident; $direction:ident; $($wname:ident),*) => {
        impl_widget_layout!($ty<>; $direction; $($wname),*);
    };
}
