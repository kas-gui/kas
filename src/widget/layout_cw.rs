//! Macros using the Cassowary constraint solver

#[macro_export]
macro_rules! cw_var {
    ($w:expr, w) => { $crate::cw::Variable::from_usize($w.get_number() as usize) };
    ($w:expr, h) => { $crate::cw::Variable::from_usize(($w.get_number() + 0x1000_0000) as usize) };
}

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

// TODO: borders and margins
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
