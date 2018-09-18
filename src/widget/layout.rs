//! Widget layout

use cw;

use std::fmt;

use Coord;
use widget::WidgetCore;
use toolkit::Toolkit;

pub trait Layout: WidgetCore + fmt::Debug {
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
    fn init_constraints(&self, tk: &Toolkit,
        s: &mut cw::Solver, use_default: bool);
    
    /// Apply constraints from the solver.
    /// 
    /// See the `init_constraints` documentation.
    /// 
    /// `pos` is the widget's position relative to the parent window.
    fn apply_constraints(&mut self, tk: &Toolkit, s: &cw::Solver, pos: Coord);
    
    /// Read position and size of widget from the toolkit
    /// 
    /// This is for use when the toolkit does layout adjustment of widgets.
    fn sync_size(&mut self, tk: &Toolkit);
}

impl<'a, L: Layout> Layout for &'a mut L {
    fn init_constraints(&self, tk: &Toolkit,
        s: &mut cw::Solver, use_default: bool)
    {
        (**self).init_constraints(tk, s, use_default)
    }
    
    fn apply_constraints(&mut self, tk: &Toolkit, s: &cw::Solver, pos: Coord) {
        (**self).apply_constraints(tk, s, pos)
    }
    
    fn sync_size(&mut self, tk: &Toolkit) {
        (**self).sync_size(tk)
    }
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
            fn init_constraints(&self, tk: &$crate::toolkit::Toolkit,
                s: &mut $crate::cw::Solver, use_default: bool)
            {
                use $crate::cw;
                
                let key = self.get_number();
                let v0 = cw::Variable::from_usize(key);
                let v1 = cw::Variable::from_usize(key + 0x1000_0000);
                
                let (min, hint) = tk.tk_widget().size_hints(self.get_tkd());
                
                // minimum size constraints:
                s.add_constraint(cw::Constraint::new(
                    cw::Expression::from_constant(min.0 as f64) - v0,
                    cw::RelationalOperator::LessOrEqual,
                    cw::strength::STRONG)).unwrap();
                s.add_constraint(cw::Constraint::new(
                    cw::Expression::from_constant(min.1 as f64) - v1,
                    cw::RelationalOperator::LessOrEqual,
                    cw::strength::STRONG)).unwrap();
                
                // preferred size constraints:
                s.add_constraint(cw::Constraint::new(
                    cw::Expression::from_constant(hint.0 as f64) - v0,
                    cw::RelationalOperator::LessOrEqual,
                    cw::strength::MEDIUM)).unwrap();
                s.add_constraint(cw::Constraint::new(
                    cw::Expression::from_constant(hint.1 as f64) - v1,
                    cw::RelationalOperator::LessOrEqual,
                    cw::strength::MEDIUM)).unwrap();
                
                /*
                // starting points:
                let size = if use_default { hint } else { self.rect().size };
                s.add_edit_variable(v0, cw::strength::WEAK).unwrap();
                s.suggest_value(v0, size.0 as f64);
                s.add_edit_variable(v1, cw::strength::WEAK).unwrap();
                s.suggest_value(v1, size.1 as f64);
                */
            }
            
            fn apply_constraints(&mut self, tk: &$crate::toolkit::Toolkit,
                s: &$crate::cw::Solver, pos: $crate::Coord) 
            {
                let key = self.get_number();
                let v0 = $crate::cw::Variable::from_usize(key);
                let v1 = $crate::cw::Variable::from_usize(key + 0x1000_0000);
                
                let tkd = self.get_tkd();
                let size = (s.get_value(v0) as i32, s.get_value(v1) as i32);
                let rect = self.rect_mut();
                rect.pos = pos;
                rect.size = size;
                tk.tk_widget().set_rect(tkd, rect);
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
            fn init_constraints(&self, tk: &$crate::toolkit::Toolkit,
                s: &mut $crate::cw::Solver, use_default: bool)
            {
                self.$child.init_constraints(tk, s, use_default)
            }
            
            fn apply_constraints(&mut self, tk: &$crate::toolkit::Toolkit,
                s: &$crate::cw::Solver, pos: $crate::Coord)
            {
                self.$child.apply_constraints(tk, s, pos)
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
macro_rules! count_items {
    ($name:ident) => { 1 };
    ($first:ident, $($rest:ident),*) => {
        1 + count_items!($($rest),*)
    }
}

/// Construct a container widget
#[macro_export]
macro_rules! make_layout {
    ($direction:ident;
        $($wname:ident $wt:ident : $wvalue:expr),* ;
        $($dname:ident $dt:ident : $dvalue:expr),* ;) =>
    {{
        use std::fmt::{self, Debug};
        use $crate::widget::{Class, CoreData, WidgetCore, Widget, Layout};
        use $crate::event::{Event, Handler, Response};
        use $crate::toolkit::Toolkit;
        use $crate::cw;

        #[derive(Clone)]
        struct L<$($wt: Widget + 'static),* , $($dt),*> {
            core: CoreData,
            $($wname: $wt),* ,
            $($dname: $dt),*
        }

        impl_widget_core!(L<$($wt: Widget),* , $($dt),*>, core);

        impl<$($wt: Widget),* , $($dt),*> Debug
            for L<$($wt),* , $($dt),*>
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "<widget> {{ core: {:?}", self.core)?;
                $(write!(f, ", {}: {:?}", stringify!($wname), self.$wname)?;)*
                $(write!(f, ", {}: <omitted>", stringify!($dname))?;)*
                write!(f, " }}")
            }
        }

        impl<$($wt: Widget),* , $($dt),*> Layout
            for L<$($wt),* , $($dt),*>
        {
            fn init_constraints(&self, tk: &Toolkit,
                s: &mut cw::Solver, use_default: bool)
            {
                let key = self.get_number();
                let v0 = cw::Variable::from_usize(key);
                let v1 = cw::Variable::from_usize(key + 0x1000_0000);
                
                // TODO: borders and margins
                
                let width = cw::Expression::from(v0);
                let mut height = cw::Expression::from(v1);
                
                $(
                    let child_v0 = cw::Variable::from_usize(key);
                    let child_v1 = cw::Variable::from_usize(key + 1);
                    s.add_constraint(cw::Constraint::new(
                        width.clone() - child_v0,
                        cw::RelationalOperator::GreaterOrEqual,
                        cw::strength::STRONG)).unwrap();
                    s.add_constraint(cw::Constraint::new(
                        width.clone() - child_v0,
                        cw::RelationalOperator::Equal,
                        cw::strength::MEDIUM)).unwrap();
                    height -= child_v1;
                    self.$wname.init_constraints(tk, s, use_default);
                )*
                
                s.add_constraint(cw::Constraint::new(
                    height,
                    cw::RelationalOperator::Equal,
                    cw::strength::STRONG * 10.0)).unwrap();
            }
            
            fn apply_constraints(&mut self, tk: &Toolkit,
                s: &cw::Solver, mut pos: $crate::Coord)
            {
                let key = self.get_number();
                let v0 = cw::Variable::from_usize(key);
                let v1 = cw::Variable::from_usize(key + 0x1000_0000);
                
                let tkd = self.get_tkd();
                let size = (s.get_value(v0) as i32, s.get_value(v1) as i32);
                {
                    let rect = self.rect_mut();
                    rect.pos = pos;
                    rect.size = size;
                    tk.tk_widget().set_rect(tkd, rect);
                }
                
                $(
                    let child_v1 = cw::Variable::from_usize(
                        self.$wname.get_number() + 0x1000_0000);
                    self.$wname.apply_constraints(tk, s, pos);
                    pos.1 += s.get_value(child_v1) as i32;
                )*
            }
            
            fn sync_size(&mut self, tk: &Toolkit) {
                let new_rect = tk.tk_widget().get_rect(self.get_tkd());
                *self.rect_mut() = new_rect;
                
                $(self.$wname.sync_size(tk);)*
            }
        }

        impl<$($wt: Widget + 'static),* , $($dt),*> Widget
            for L<$($wt),* , $($dt),*>
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

        impl<$($wt: Widget),* , $($dt),*> Handler
            for L<$($wt),* , $($dt),*>
        {
            type Response = Response;   // TODO
            fn handle(&mut self, _event: Event) -> Self::Response {
                unimplemented!()
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
