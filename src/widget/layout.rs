//! Widget layout

use std::mem;

use cw;

use Coord;
use widget::WidgetCore;
use toolkit::Toolkit;

pub trait Layout: WidgetCore {
    /// Upcast
    fn as_core(&self) -> &WidgetCore;
    /// Upcast, mutably
    fn as_core_mut(&mut self) -> &mut WidgetCore;
    
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
    /// 
    /// `key` is used to provide a unique identifier for each constraint
    /// variable. Widgets must use `key` for their width and `key + 1` for their
    /// height, and may use values if more variables are needed. Widgets must
    /// return the next unused `key` value (typically `key + 2` if there are no
    /// child widgets). Behaviour must be identical in `apply_constraints` in
    /// order to reproduce the same `key` values.
    // TODO: because of width-for-height relations it may be necessary to
    // adjust this, e.g. solve for width first then for height.
    fn init_constraints(&self, tk: &Toolkit, key: usize,
        s: &mut cw::Solver, use_default: bool) -> usize
    {
        let v0 = cw::Variable::from_usize(key);
        let v1 = cw::Variable::from_usize(key + 1);
        let key = key + 2;
        
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
        
        key
    }
    
    /// Apply constraints from the solver.
    /// 
    /// See the `init_constraints` documentation.
    /// 
    /// `pos` is the widget's position relative to the parent window.
    fn apply_constraints(&mut self, tk: &Toolkit, key: usize,
        s: &cw::Solver, pos: Coord) -> usize
    {
        let v0 = cw::Variable::from_usize(key);
        let v1 = cw::Variable::from_usize(key + 1);
        let key = key + 2;
        
        let tkd = self.as_core().get_tkd();
        let size = (s.get_value(v0) as i32, s.get_value(v1) as i32);
        let rect = self.as_core_mut().rect_mut();
        rect.pos = pos;
        rect.size = size;
        tk.tk_widget().set_rect(tkd, rect);
        
        key
    }
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
        use $crate::widget::{Class, CoreData, WidgetCore, Widget, Layout};
        use $crate::event::{Event, Handler, Response};
        use $crate::toolkit::Toolkit;
        use $crate::cw;

        struct L<$($wt: Widget + 'static),* , $($dt),*> {
            core: CoreData,
            $($wname: $wt),* ,
            $($dname: $dt),*
        }

        impl_widget_core!(L<$($wt: Widget),* , $($dt),*>, core);

        impl<$($wt: Widget),* , $($dt),*> Layout
            for L<$($wt),* , $($dt),*>
        {
            fn as_core(&self) -> &WidgetCore { &self.core }
            fn as_core_mut(&mut self) -> &mut WidgetCore { &mut self.core }
            
            fn init_constraints(&self, tk: &Toolkit, key: usize,
                s: &mut cw::Solver, use_default: bool) -> usize
            {
                let v0 = cw::Variable::from_usize(key);
                let v1 = cw::Variable::from_usize(key + 1);
                let mut key = key + 2;
                
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
                    key = self.$wname.init_constraints(tk, key, s, use_default);
                )*
                
                s.add_constraint(cw::Constraint::new(
                    height,
                    cw::RelationalOperator::Equal,
                    cw::strength::STRONG * 10.0)).unwrap();
                
                key
            }
            
            /// Apply constraints from the solver.
            /// 
            /// See the `init_constraints` documentation.
            /// 
            /// `pos` is the widget's position relative to the parent window.
            fn apply_constraints(&mut self, tk: &Toolkit, key: usize,
                s: &cw::Solver, mut pos: $crate::Coord) -> usize
            {
                let v0 = cw::Variable::from_usize(key);
                let v1 = cw::Variable::from_usize(key + 1);
                let mut key = key + 2;
                
                let tkd = self.as_core().get_tkd();
                let size = (s.get_value(v0) as i32, s.get_value(v1) as i32);
                {
                    let rect = self.as_core_mut().rect_mut();
                    rect.pos = pos;
                    rect.size = size;
                    tk.tk_widget().set_rect(tkd, rect);
                }
                
                $(
                    let child_v1 = cw::Variable::from_usize(key + 1);
                    key = self.$wname.apply_constraints(tk, key, s, pos);
                    pos.1 += s.get_value(child_v1) as i32;
                )*
                
                key
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
