// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::Ident;
use syn::parse::{Error, Result};
use crate::args::Child;

pub(crate) fn fns(c: &TokenStream, children: &Vec<Child>, layout: Option<Ident>)
    -> Result<TokenStream>
{
    let (constraints, appls) = if children.is_empty() {
        // TODO: warn on invalid layout specification
        (quote!{
            let v_w = #c::cw_var!(self, w);
            let v_h = #c::cw_var!(self, h);
            
            let (min, hint) = tk.size_hints(self.tkd());
            
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
            let size = if _use_default { hint } else { self.rect().size };
            s.add_edit_variable(v_w, cw::strength::WEAK).unwrap();
            s.suggest_value(v_w, size.0 as f64);
            s.add_edit_variable(v_h, cw::strength::WEAK).unwrap();
            s.suggest_value(v_h, size.1 as f64);
            */
        }, quote!{})
    } else if children.len() == 1 {
        // TODO: warn on invalid layout specification
        let ident = &children[0].ident;
        (quote!{
            s.add_constraint(cw::Constraint::new(
                cw::Expression::from(#c::cw_var!(self, w)) - #c::cw_var!(self.#ident, w),
                cw::RelationalOperator::Equal,
                cw::strength::STRONG)).unwrap();
            s.add_constraint(cw::Constraint::new(
                cw::Expression::from(#c::cw_var!(self, h)) - #c::cw_var!(self.#ident, h),
                cw::RelationalOperator::Equal,
                cw::strength::STRONG)).unwrap();
            self.#ident.init_constraints(tk, s, _use_default);
        }, quote!{})
    } else {
        if let Some(l) = layout {
            if l == "horizontal" {
                let mut constr = quote!{
                    let mut width = cw::Expression::from(#c::cw_var!(self, w));
                    let height = cw::Expression::from(#c::cw_var!(self, h));
                };
                let mut appls = quote!{ let mut cpos = pos; };
                
                for child in children {
                    let ident = &child.ident;
                    
                    constr.append_all(quote!{
                        let child_v_w = #c::cw_var!(self.#ident, w);
                        let child_v_h = #c::cw_var!(self.#ident, h);
                        width -= child_v_w;
                        s.add_constraint(cw::Constraint::new(
                            height.clone() - child_v_h,
                            cw::RelationalOperator::GreaterOrEqual,
                            cw::strength::STRONG)).unwrap();
                        s.add_constraint(cw::Constraint::new(
                            height.clone() - child_v_h,
                            cw::RelationalOperator::Equal,
                            cw::strength::MEDIUM)).unwrap();
                        self.#ident.init_constraints(tk, s, _use_default);
                    });
                    
                    appls.append_all(quote!{
                        self.#ident.apply_constraints(tk, s, cpos);
                        cpos.0 += s.get_value(#c::cw_var!(self.#ident, w)) as i32;
                    });
                }
                
                constr.append_all(quote!{
                    s.add_constraint(cw::Constraint::new(
                        width,
                        cw::RelationalOperator::Equal,
                        cw::strength::STRONG * 10.0)).unwrap();
                });
                
                (constr, appls)
            } else if l == "vertical" {
                let mut constr = quote!{
                    let width = cw::Expression::from(#c::cw_var!(self, w));
                    let mut height = cw::Expression::from(#c::cw_var!(self, h));
                };
                let mut appls = quote!{ let mut cpos = pos; };
                
                for child in children {
                    let ident = &child.ident;
                    
                    constr.append_all(quote!{
                        let child_v_w = #c::cw_var!(self.#ident, w);
                        let child_v_h = #c::cw_var!(self.#ident, h);
                        s.add_constraint(cw::Constraint::new(
                            width.clone() - child_v_w,
                            cw::RelationalOperator::GreaterOrEqual,
                            cw::strength::STRONG)).unwrap();
                        s.add_constraint(cw::Constraint::new(
                            width.clone() - child_v_w,
                            cw::RelationalOperator::Equal,
                            cw::strength::MEDIUM)).unwrap();
                        height -= child_v_h;
                        self.#ident.init_constraints(tk, s, _use_default);
                    });
                    
                    appls.append_all(quote!{
                        self.#ident.apply_constraints(tk, s, cpos);
                        cpos.1 += s.get_value(#c::cw_var!(self.#ident, h)) as i32;
                    });
                }
                
                constr.append_all(quote!{
                    s.add_constraint(cw::Constraint::new(
                        height,
                        cw::RelationalOperator::Equal,
                        cw::strength::STRONG * 10.0)).unwrap();
                });
                
                (constr, appls)
            } else if l == "grid" {
                panic!("not yet implemented: grid layout (with cassowary layout feature)")
            } else {
                return Err(Error::new(l.span(),
                    "expected one of: horizontal, vertical, grid"));
            }
        } else {
            panic!("missing layout specification")  // should already be trapped
        }
    };
    Ok(quote! {
        fn init_constraints(&self, tk: &#c::TkWidget,
            s: &mut #c::cw::Solver, _use_default: bool)
        {
            use #c::cw;
            #constraints
        }
        
        fn apply_constraints(&mut self, tk: &#c::TkWidget,
            s: &#c::cw::Solver, pos: #c::Coord)
        {
            #appls
            
            let w = s.get_value(#c::cw_var!(self, w)) as i32;
            let h = s.get_value(#c::cw_var!(self, h)) as i32;
            let tkd = self.tkd();
            let rect = self.rect_mut();
            rect.pos = pos;
            rect.size = (w, h);
            tk.set_rect(tkd, rect);
        }
    })
}
