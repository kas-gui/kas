// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::Child;
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::parse::{Error, Result};
use syn::Ident;

pub(crate) fn fns(children: &Vec<Child>, layout: Option<Ident>) -> Result<TokenStream> {
    let mut constraints;
    let mut appls;
    if children.is_empty() {
        // TODO: warn on invalid layout specification
        constraints = quote! {
            let v_w = kas::cw_var!(self, w);
            let v_h = kas::cw_var!(self, h);

            let (min, hint) = tk.size_hints(self);

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
        };
        appls = quote! {};
    } else if children.len() == 1 {
        // TODO: warn on invalid layout specification
        let ident = &children[0].ident;
        constraints = quote! {
            s.add_constraint(cw::Constraint::new(
                cw::Expression::from(kas::cw_var!(self, w)) - kas::cw_var!(self.#ident, w),
                cw::RelationalOperator::Equal,
                cw::strength::STRONG)).unwrap();
            s.add_constraint(cw::Constraint::new(
                cw::Expression::from(kas::cw_var!(self, h)) - kas::cw_var!(self.#ident, h),
                cw::RelationalOperator::Equal,
                cw::strength::STRONG)).unwrap();
            self.#ident.init_constraints(tk, s, _use_default);
        };
        appls = quote! {
            self.#ident.apply_constraints(tk, s, pos);
        };
    } else {
        if let Some(l) = layout {
            if l == "horizontal" {
                constraints = quote! {
                    let mut width = cw::Expression::from(kas::cw_var!(self, w));
                    let height = cw::Expression::from(kas::cw_var!(self, h));
                };
                appls = quote! { let mut cpos = pos; };

                for child in children {
                    let ident = &child.ident;

                    constraints.append_all(quote! {
                        let child_v_w = kas::cw_var!(self.#ident, w);
                        let child_v_h = kas::cw_var!(self.#ident, h);
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

                    appls.append_all(quote! {
                        self.#ident.apply_constraints(tk, s, cpos);
                        cpos.0 += s.get_value(kas::cw_var!(self.#ident, w)) as i32;
                    });
                }

                constraints.append_all(quote! {
                    s.add_constraint(cw::Constraint::new(
                        width,
                        cw::RelationalOperator::Equal,
                        cw::strength::STRONG * 10.0)).unwrap();
                });
            } else if l == "vertical" {
                constraints = quote! {
                    let width = cw::Expression::from(kas::cw_var!(self, w));
                    let mut height = cw::Expression::from(kas::cw_var!(self, h));
                };
                appls = quote! { let mut cpos = pos; };

                for child in children {
                    let ident = &child.ident;

                    constraints.append_all(quote! {
                        let child_v_w = kas::cw_var!(self.#ident, w);
                        let child_v_h = kas::cw_var!(self.#ident, h);
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

                    appls.append_all(quote! {
                        self.#ident.apply_constraints(tk, s, cpos);
                        cpos.1 += s.get_value(kas::cw_var!(self.#ident, h)) as i32;
                    });
                }

                constraints.append_all(quote! {
                    s.add_constraint(cw::Constraint::new(
                        height,
                        cw::RelationalOperator::Equal,
                        cw::strength::STRONG * 10.0)).unwrap();
                });
            } else if l == "grid" {
                // TODO(opt): uses two redundant variables per dimension

                let (mut mw, mut mh) = (0, 0);

                constraints = quote! {};
                appls = quote! {};

                for (ci, child) in children.iter().enumerate() {
                    let ident = &child.ident;
                    let gpos = child.args.as_pos()?;
                    //println!("Child {}, grid pos {:?}", ci, gpos);

                    let (w0, h0) = (gpos.0, gpos.1);
                    let (w1, h1) = (w0 + gpos.2, h0 + gpos.3);
                    mw = mw.max(w1);
                    mh = mh.max(h1);

                    constraints.append_all(quote! {
                        let mut width = cw::Expression::from(kas::cw_var!(self.#ident, w))
                            + kas::cw_var!(self, w, #w0)
                            - kas::cw_var!(self, w, #w1);
                        let mut height = cw::Expression::from(kas::cw_var!(self.#ident, h))
                            + kas::cw_var!(self, h, #h0)
                            - kas::cw_var!(self, h, #h1);

                        s.add_constraint(cw::Constraint::new(
                            width.clone(),
                            cw::RelationalOperator::LessOrEqual,
                            cw::strength::STRONG)).unwrap();
                        s.add_constraint(cw::Constraint::new(
                            width,
                            cw::RelationalOperator::Equal,
                            cw::strength::MEDIUM)).unwrap();

                        s.add_constraint(cw::Constraint::new(
                            height.clone(),
                            cw::RelationalOperator::LessOrEqual,
                            cw::strength::STRONG)).unwrap();
                        s.add_constraint(cw::Constraint::new(
                            height,
                            cw::RelationalOperator::Equal,
                            cw::strength::MEDIUM)).unwrap();
                    });

                    appls.append_all(quote! {
                        let cpos = (
                            pos.0 + s.get_value(kas::cw_var!(self, w, #w0)) as i32,
                            pos.1 + s.get_value(kas::cw_var!(self, h, #h0)) as i32);
                        //println!("Child {}, Grid ({}, {}), position {:?}", #ci, #w0, #h0, cpos);
                        self.#ident.apply_constraints(tk, s, cpos);
                    });
                }

                constraints.append_all(quote! {
                    s.add_constraint(cw::Constraint::new(
                        cw::Expression::from(kas::cw_var!(self, w, 0)),
                        cw::RelationalOperator::Equal,
                        cw::strength::STRONG * 10.0)).unwrap();
                    s.add_constraint(cw::Constraint::new(
                        cw::Expression::from(kas::cw_var!(self, h, 0)),
                        cw::RelationalOperator::Equal,
                        cw::strength::STRONG * 10.0)).unwrap();
                });
                for i in 0..mw {
                    constraints.append_all(quote!{
                    s.add_constraint(cw::Constraint::new(
                        cw::Expression::from(kas::cw_var!(self, w, #i + 1)) - kas::cw_var!(self, w, #i),
                        cw::RelationalOperator::GreaterOrEqual,
                        cw::strength::STRONG * 10.0)).unwrap();
                    });
                }
                for i in 0..mh {
                    constraints.append_all(quote!{
                    s.add_constraint(cw::Constraint::new(
                        cw::Expression::from(kas::cw_var!(self, h, #i + 1)) - kas::cw_var!(self, h, #i),
                        cw::RelationalOperator::GreaterOrEqual,
                        cw::strength::STRONG * 10.0)).unwrap();
                    });
                }
                constraints.append_all(quote! {
                    s.add_constraint(cw::Constraint::new(
                        cw::Expression::from(kas::cw_var!(self, w, #mw)) - kas::cw_var!(self, w),
                        cw::RelationalOperator::Equal,
                        cw::strength::STRONG * 10.0)).unwrap();
                    s.add_constraint(cw::Constraint::new(
                        cw::Expression::from(kas::cw_var!(self, h, #mh)) - kas::cw_var!(self, h),
                        cw::RelationalOperator::Equal,
                        cw::strength::STRONG * 10.0)).unwrap();
                });
            } else {
                return Err(Error::new(
                    l.span(),
                    "expected one of: horizontal, vertical, grid",
                ));
            }
        } else {
            panic!("missing layout specification") // should already be trapped
        }
    };
    Ok(quote! {
        fn init_constraints(&self, tk: &kas::TkWidget,
            s: &mut kas::cw::Solver, _use_default: bool)
        {
            use kas::{Core, cw};
            #constraints
        }

        fn apply_constraints(&mut self, tk: &mut kas::TkWidget,
            s: &kas::cw::Solver, pos: kas::Coord)
        {
            use kas::{Core, Rect, Size};
            //println!("Pos: {:?}", pos);
            #appls

            let size = Size(
                s.get_value(kas::cw_var!(self, w)) as u32,
                s.get_value(kas::cw_var!(self, h)) as u32);
            self.core_data_mut().rect = Rect { pos, size };
        }
    })
}
