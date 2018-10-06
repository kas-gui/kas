//! Widget layout


use std::fmt;

#[cfg(feature = "cassowary")] use crate::cw;
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

#[cfg(feature = "cassowary")]
#[macro_export]
macro_rules! cw_var {
    ($w:expr, w) => { $crate::cw::Variable::from_usize($w.get_number() as usize) };
    ($w:expr, h) => { $crate::cw::Variable::from_usize(($w.get_number() + 0x1000_0000) as usize) };
}

#[cfg(not(feature = "cassowary"))]
#[macro_export]
macro_rules! layout_init_constraints_simple {
    () => {}
}

#[cfg(feature = "cassowary")]
#[macro_export]
macro_rules! layout_init_constraints_simple {
    () => {
        fn init_constraints(&self, tk: &$crate::toolkit::Toolkit,
            s: &mut $crate::cw::Solver, use_default: bool)
        {
            use $crate::cw;
            
            let v_w = cw_var!(self, w);
            let v_h = cw_var!(self, h);
            
            let (min, hint) = tk.tk_widget().size_hints(self.get_tkd());
            
            // minimum size constraints:
            s.add_constraint(cw::Constraint::new(
                cw::Expression::from_constant(min.0 as f64) - v_w,
                cw::RelationalOperator::LessOrEqual,
                cw::strength::STRONG)).unwrap();
            s.add_constraint(cw::Constraint::new(
                cw::Expression::from_constant(min.1 as f64) - v_h,
                cw::RelationalOperator::LessOrEqual,
                cw::strength::STRONG)).unwrap();
            
            // preferred size constraints:
            s.add_constraint(cw::Constraint::new(
                cw::Expression::from_constant(hint.0 as f64) - v_w,
                cw::RelationalOperator::LessOrEqual,
                cw::strength::MEDIUM)).unwrap();
            s.add_constraint(cw::Constraint::new(
                cw::Expression::from_constant(hint.1 as f64) - v_h,
                cw::RelationalOperator::LessOrEqual,
                cw::strength::MEDIUM)).unwrap();
            
            /*
            // starting points:
            let size = if use_default { hint } else { self.rect().size };
            s.add_edit_variable(v_w, cw::strength::WEAK).unwrap();
            s.suggest_value(v_w, size.0 as f64);
            s.add_edit_variable(v_h, cw::strength::WEAK).unwrap();
            s.suggest_value(v_h, size.1 as f64);
            */
        }
    }
}

#[cfg(not(feature = "cassowary"))]
#[macro_export]
macro_rules! layout_init_constraints {
    ($direction:ident; $($wname:ident),*) => {}
}

// TODO: borders and margins
#[cfg(feature = "cassowary")]
#[macro_export]
macro_rules! layout_init_constraints {
    (single; $($wname:ident),*) => {
        fn init_constraints(&self, tk: &$crate::toolkit::Toolkit,
            s: &mut $crate::cw::Solver, use_default: bool)
        {
            use $crate::cw;
            $(
                s.add_constraint(cw::Constraint::new(
                    cw::Expression::from(cw_var!(self, w)) - cw_var!(self.$wname, w),
                    cw::RelationalOperator::Equal,
                    cw::strength::STRONG)).unwrap();
                s.add_constraint(cw::Constraint::new(
                    cw::Expression::from(cw_var!(self, h)) - cw_var!(self.$wname, h),
                    cw::RelationalOperator::Equal,
                    cw::strength::STRONG)).unwrap();
                self.$wname.init_constraints(tk, s, use_default);
            )*
        }
    };
    (horizontal; $($wname:ident),*) => {
        fn init_constraints(&self, tk: &$crate::toolkit::Toolkit,
            s: &mut $crate::cw::Solver, use_default: bool)
        {
            use $crate::cw;
            let mut width = cw::Expression::from(cw_var!(self, w));
            let height = cw::Expression::from(cw_var!(self, h));
            
            $(
                let child_v_w = cw_var!(self.$wname, w);
                let child_v_h = cw_var!(self.$wname, h);
                width -= child_v_w;
                s.add_constraint(cw::Constraint::new(
                    height.clone() - child_v_h,
                    cw::RelationalOperator::GreaterOrEqual,
                    cw::strength::STRONG)).unwrap();
                s.add_constraint(cw::Constraint::new(
                    height.clone() - child_v_h,
                    cw::RelationalOperator::Equal,
                    cw::strength::MEDIUM)).unwrap();
                self.$wname.init_constraints(tk, s, use_default);
            )*
            
            s.add_constraint(cw::Constraint::new(
                width,
                cw::RelationalOperator::Equal,
                cw::strength::STRONG * 10.0)).unwrap();
        }
    };
    (vertical; $($wname:ident),*) => {
        fn init_constraints(&self, tk: &$crate::toolkit::Toolkit,
            s: &mut $crate::cw::Solver, use_default: bool)
        {
            use $crate::cw;
            let width = cw::Expression::from(cw_var!(self, w));
            let mut height = cw::Expression::from(cw_var!(self, h));
            
            $(
                let child_v_w = cw_var!(self.$wname, w);
                let child_v_h = cw_var!(self.$wname, h);
                s.add_constraint(cw::Constraint::new(
                    width.clone() - child_v_w,
                    cw::RelationalOperator::GreaterOrEqual,
                    cw::strength::STRONG)).unwrap();
                s.add_constraint(cw::Constraint::new(
                    width.clone() - child_v_w,
                    cw::RelationalOperator::Equal,
                    cw::strength::MEDIUM)).unwrap();
                height -= child_v_h;
                self.$wname.init_constraints(tk, s, use_default);
            )*
            
            s.add_constraint(cw::Constraint::new(
                height,
                cw::RelationalOperator::Equal,
                cw::strength::STRONG * 10.0)).unwrap();
        }
    };
}

#[cfg(not(feature = "cassowary"))]
#[macro_export]
macro_rules! layout_apply_constraints {
    ($direction:ident; $($wname:ident),*) => {}
}

#[cfg(feature = "cassowary")]
#[macro_export]
macro_rules! layout_apply_constraints {
    ($direction:ident; $($wname:ident),*) => {
        fn apply_constraints(&mut self, tk: &$crate::toolkit::Toolkit,
            s: &$crate::cw::Solver, pos: $crate::Coord)
        {
            let mut cpos = pos;
            $(
                self.$wname.apply_constraints(tk, s, cpos);
                layout_apply_constraints_next!($direction; self, s, cpos; $wname);
            )*
            
            let w = s.get_value(cw_var!(self, w)) as i32;
            let h = s.get_value(cw_var!(self, h)) as i32;
            let tkd = self.get_tkd();
            let rect = self.rect_mut();
            rect.pos = pos;
            rect.size = (w, h);
            tk.tk_widget().set_rect(tkd, rect);
        }
    }
}

#[cfg(feature = "cassowary")]
#[macro_export]
macro_rules! layout_apply_constraints_next {
    (single; $self:ident, $s:ident, $pos:ident; $wname:ident) => {};
    (horizontal; $self:ident, $s:ident, $pos:ident; $wname:ident) => {
        $pos.0 += $s.get_value(cw_var!($self.$wname, w)) as i32;
    };
    (vertical; $self:ident, $s:ident, $pos:ident; $wname:ident) => {
        $pos.1 += $s.get_value(cw_var!($self.$wname, h)) as i32;
    };
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
