// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data types specific to the layout engine

use std::fmt;

use super::Rect;
use crate::toolkit::TkWindow;
use crate::Core;

/// Used by the layout engine to specify the axis of interest.
///
/// The layout engine works on a single axis at a time, and when doing so may
/// provide a fixed size for the other axis.
#[derive(Copy, Clone, Debug)]
pub struct AxisInfo {
    is_vert: bool,
    fixed: bool,
    other: u32,
}

impl AxisInfo {
    /// Construct an instance
    #[inline]
    pub fn new(vert: bool, fixed: Option<u32>) -> Self {
        AxisInfo {
            is_vert: vert,
            fixed: fixed.is_some(),
            other: fixed.unwrap_or(0),
        }
    }

    /// Adjust horizontal axis
    #[inline]
    pub fn horiz(&self) -> bool {
        !self.is_vert
    }

    /// Adjust vertical axis
    #[inline]
    pub fn vert(&self) -> bool {
        self.is_vert
    }

    /// Size of other axis, if fixed and (`vert == self.vert()`).
    #[inline]
    pub fn fixed(&self, vert: bool) -> Option<u32> {
        if vert == self.is_vert && self.fixed {
            Some(self.other)
        } else {
            None
        }
    }

    /// Set size of fixed axis, if applicable
    #[inline]
    pub fn set_size(&mut self, size: u32) {
        self.other = size;
    }
}

/// Return value of [`Layout::size_rules`].
///
/// This struct conveys properties such as the minimum size and preferred size
/// of the widgets being queried.
#[derive(Copy, Clone, Debug, Default)]
pub struct SizeRules {
    // minimum size
    a: u32,
    // maximum size; b >= a
    b: u32,
}

impl SizeRules {
    /// Empty (zero size)
    pub const EMPTY: Self = SizeRules { a: 0, b: 0 };

    /// A fixed size
    #[inline]
    pub fn fixed(size: u32) -> Self {
        SizeRules { a: size, b: size }
    }

    /// A variable size with given `min`-imum and `pref`-erred values.
    ///
    /// Required: `pref >= min`.
    #[inline]
    pub fn variable(min: u32, pref: u32) -> Self {
        if min > pref {
            panic!("SizeRules::variable(min, pref): min > pref !");
        }
        SizeRules { a: min, b: pref }
    }

    /// Use the maximum size of `self` and `rhs`.
    #[inline]
    pub fn max(self, rhs: Self) -> SizeRules {
        SizeRules {
            a: self.a.max(rhs.a),
            b: self.b.max(rhs.b),
        }
    }

    /// Like `self = self.max(x - y)` but handling negative values correctly
    // TODO: switch to i32?
    pub fn set_at_least_op_sub(&mut self, x: Self, y: Self) {
        if x.a > y.a {
            self.a = self.a.max(x.a - y.a);
        }
        if x.b > y.b {
            self.b = self.b.max(x.b - y.b);
        }
    }

    #[doc(hidden)]
    /// Solve a sequence of rules
    ///
    /// Given a sequence of width / height `rules` from children (including a
    /// final value which is the total) and a `target` size, find an appropriate
    /// size for each child width / height.
    // TODO (const generics):
    // fn solve_seq<const N: usize>(out: &mut [u32; N], rules: &[Self; N + 1], target: u32)
    pub fn solve_seq(out: &mut [u32], rules: &[Self], target: u32) {
        #[allow(non_snake_case)]
        let N = out.len();
        assert!(rules.len() == N + 1);
        if N == 0 {
            return;
        }

        if target >= rules[N].a {
            // At or over minimum: distribute extra relative to preferences.
            // TODO: perhaps this should not use the minimum except as a minimum?

            let target_rel = target - rules[N].a;
            let pref_rel = rules[N].b - rules[N].a;
            let mut sum = 0;

            if pref_rel > 0 {
                let x = target_rel as f64 / pref_rel as f64;

                for n in 0..N {
                    // This will round down:
                    let r = rules[n];
                    let size = r.a + (x * (r.b - r.a) as f64) as u32;
                    out[n] = size;
                    sum += size;
                }
            } else {
                // special case: pref_rel == 0
                let add = target_rel / N as u32;
                for n in 0..N {
                    let size = rules[n].a + add;
                    out[n] = size;
                    sum += size;
                }
            }

            // The above may round down, which may leave us a little short.
            assert!(sum <= target);
            let rem = target - sum;
            assert!(rem as usize <= N);
            // Distribute to first rem. sizes.
            for n in 0..(rem as usize) {
                out[n] += 1;
            }
        } else {
            // Under minimum: reduce maximum allowed size.
            let mut excess = rules[N].a - target;

            let mut largest = 0;
            let mut num_equal = 0;
            let mut next_largest = 0;
            for n in 0..N {
                let a = rules[n].a;
                out[n] = a;
                if a == largest {
                    num_equal += 1;
                } else if a > largest {
                    next_largest = largest;
                    largest = a;
                    num_equal = 1;
                } else if a > next_largest {
                    next_largest = a;
                }
            }

            while excess > 0 {
                let step = (excess / num_equal).min(largest - next_largest);
                if step == 0 {
                    for n in 0..N {
                        if out[n] == largest {
                            out[n] -= 1;
                            if excess == 0 {
                                break;
                            }
                            excess -= 1;
                        }
                    }
                    break;
                }

                let thresh = next_largest;
                let mut num_add = 0;
                next_largest = 0;
                for n in 0..N {
                    let a = out[n];
                    if a == largest {
                        out[n] = a - step;
                    } else if a == thresh {
                        num_add += 1;
                    } else if a > next_largest {
                        next_largest = a;
                    }
                }
                excess -= step * num_equal;

                largest -= step;
                num_equal += num_add;
            }
        }
    }
}

impl std::ops::Add<SizeRules> for SizeRules {
    type Output = Self;

    #[inline]
    fn add(self, rhs: SizeRules) -> Self::Output {
        SizeRules {
            a: self.a + rhs.a,
            b: self.b + rhs.b,
        }
    }
}

impl std::ops::AddAssign for SizeRules {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = Self {
            a: self.a + rhs.a,
            b: self.b + rhs.b,
        };
    }
}

/// Widget size and layout.
pub trait Layout: Core + fmt::Debug {
    /// Get size rules for the given axis.
    ///
    /// This method takes `&mut self` to allow local caching of child widget
    /// configuration for future `size_rules` and `set_rect` calls.
    ///
    /// If operating on one axis and the other is fixed, then the `other`
    /// parameter is used for the fixed dimension. Additionally, one may assume
    /// that `size_rules` has previously been called on the fixed axis with the
    /// current widget configuration.
    fn size_rules(&mut self, tk: &mut dyn TkWindow, axis: AxisInfo) -> SizeRules;

    /// Adjust to the given size.
    ///
    /// For many widgets this operation is trivial and the default
    /// implementation will suffice. For layout widgets (those with children),
    /// this operation is more complex.
    ///
    /// One may assume that `size_rules` has been called for each axis with the
    /// current widget configuration.
    #[inline]
    fn set_rect(&mut self, rect: Rect) {
        self.core_data_mut().rect = rect;
    }
}
