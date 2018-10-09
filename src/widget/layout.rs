//! Widget layout


use std::fmt;

#[cfg(feature = "cassowary")] use crate::cw;
#[cfg(feature = "cassowary")] use crate::Coord;
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
pub trait Layout: WidgetCore + fmt::Debug {
    /// Layout for child widgets
    fn child_layout(&self) -> ChildLayout;

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
    fn init_constraints(&self, tk: &Toolkit,
        s: &mut cw::Solver, use_default: bool);
    
    /// Apply constraints from the solver.
    /// 
    /// See the `init_constraints` documentation.
    /// 
    /// `pos` is the widget's position relative to the parent window.
    #[cfg(feature = "cassowary")]
    fn apply_constraints(&mut self, tk: &Toolkit, s: &cw::Solver, pos: Coord);
    
    /// Read position and size of widget from the toolkit
    /// 
    /// This is for use when the toolkit does layout adjustment of widgets.
    fn sync_size(&mut self, tk: &Toolkit);
}


#[cfg(not(feature = "cassowary"))]
#[macro_export]
macro_rules! layout_init_constraints_simple {
    () => {}
}

#[cfg(not(feature = "cassowary"))]
#[macro_export]
macro_rules! layout_init_constraints {
    ($direction:ident; $($wname:ident),*) => {}
}

#[cfg(not(feature = "cassowary"))]
#[macro_export]
macro_rules! layout_apply_constraints {
    ($direction:ident; $($wname:ident),*) => {}
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

            layout_init_constraints_simple!();
            layout_apply_constraints!(single; );
            
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

            layout_init_constraints!(single; $child);
            layout_apply_constraints!(single; $child);
            
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
macro_rules! count_items {
    ($name:ident) => { 1 };
    ($first:ident, $($rest:ident),*) => {
        1 + count_items!($($rest),*)
    }
}

#[macro_export]
macro_rules! select_child_layout {
    (single) => { $crate::widget::ChildLayout::None };
    (horizontal) => { $crate::widget::ChildLayout::Horizontal };
    (vertical) => { $crate::widget::ChildLayout::Vertical };
    (grid) => { $crate::widget::ChildLayout::Grid };
}

/// Construct a container widget
#[macro_export]
macro_rules! make_layout {
    ($direction:ident < $($gt:ident [$gtr:path]),* >;
        $self:ident, $tk: ident, $msg:ident;
        $($wname:ident: $wt:ident = $wvalue:expr => $whandle:expr),* ;
        $($dname:ident: $dt:ident = $dvalue:expr),* ;
        $response:path) =>
    {{
        use $crate::event::{Action, Handler, ignore};
        use $crate::toolkit::Toolkit;
        use $crate::widget::{Class, CoreData, WidgetCore, Widget, Layout};

        #[derive(Clone, Debug)]
        struct L<$($gt: Widget + 'static),*> {
            core: CoreData,
            $($wname: $wt),* ,
            $($dname: $dt),*
        }

        impl_widget_core!(L<$($gt: Widget),*>, core);

        impl<$($gt: Widget),*> Layout for L<$($gt),*> {
            fn child_layout(&self) -> $crate::widget::ChildLayout {
                select_child_layout!($direction)
            }

            layout_init_constraints!($direction; $($wname),*);
            layout_apply_constraints!($direction; $($wname),*);
            
            fn sync_size(&mut self, tk: &Toolkit) {
                let new_rect = tk.tk_widget().get_rect(self.get_tkd());
                *self.rect_mut() = new_rect;
                
                $(self.$wname.sync_size(tk);)*
            }
        }

        impl<$($gt: Widget + 'static),*> Widget for L<$($gt),*>
        {
            fn class(&self) -> Class { Class::Container }
            fn label(&self) -> Option<&str> { None }

            fn len(&self) -> usize {
                count_items!($($wname),*)
            }
            fn get(&self, index: usize) -> Option<&Widget> {
                // We need to match, but macros cannot expand to match arms
                // or parts of if-else chains. Hack: use direct return.
                let _i = 0;
                $(
                    if index == _i {
                        return Some(&self.$wname);
                    }
                    let _i = _i + 1;
                )*
                return None;
            }
            fn get_mut(&mut self, index: usize) -> Option<&mut Widget> {
                let _i = 0;
                $(
                    if index == _i {
                        return Some(&mut self.$wname);
                    }
                    let _i = _i + 1;
                )*
                return None;
            }
        }

        impl<$($gt: Widget + Handler<Response = $gtr>),*> Handler
            for L<$($gt),*>
        {
            type Response = $response;
            
            fn handle_action(&mut $self, $tk: &Toolkit, action: Action, num: u32) -> $response {
                $(
                    if num <= $self.$wname.get_number() {
                        let $msg = $self.$wname.handle_action($tk, action, num);
                        return $whandle;
                    }
                )*
                if num == $self.get_number() {
                    ignore(action)  // no actions handled by this widget
                } else {
                    println!("Warning: incorrect widget number");
                    ignore(action)
                }
            }
        }

        L {
            core: Default::default(),
            $($wname: $wvalue),* ,
            $($dname: $dvalue),*
        }
    }}
}

#[macro_export]
macro_rules! match_event_widget {
    ($ev:expr; $( $name:ident => $result:expr ),* , ) => {  // trailing comma
        match_event_widget!($ev; $( $name => $result ),* )
    };
    ($ev:expr; $name0:ident => $result0:expr, $( $name:ident => $result:expr ),* ) => {
        // TODO: find correct widget; currently we just assume the first
        $result0
    };
}

#[cfg(test)] mod test {
    use crate::widget::{Widget, Layout, WidgetCore, canvas::Text};
    use crate::event::NoResponse;

    #[test]
    fn macro_test_layout() {
        fn check_props<T: Widget + Layout + WidgetCore>(_x: T) {}
        
        let w = make_layout!(single<>; self, tk, msg;
            text: Text = Text::from("text") => msg;;
            NoResponse);
        check_props(w);
    }
}
