// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data types specific to the layout engine

use super::{Coord, Rect, Size};

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

/// Margin dimensions
pub struct Margins {
    /// Total size of the margin surrounding contents
    pub outer: Size,
    /// Offset of contents from widget position (usually half of `outer`)
    pub offset: Coord,
    /// Inner offset between rows / columns
    pub inner: Coord,
}

impl Margins {
    /// Construct SizeRules appropriate for the margin
    ///
    /// Parameters: `vertical` if vertical axis, number of additional
    /// `col_spacings`, `row_spacings`.
    pub fn size_rules(&self, vertical: bool, col_spacings: u32, row_spacings: u32) -> SizeRules {
        SizeRules::fixed(match vertical {
            false => self.outer.0 + col_spacings * self.inner.0 as u32,
            true => self.outer.1 + row_spacings * self.inner.1 as u32,
        })
    }

    /// Shrink and offset a `rect` to account for margins
    pub fn adjust(&self, rect: &mut Rect) {
        rect.size = rect.size - self.outer;
        rect.pos = rect.pos + self.offset;
    }
}

/// Return value of [`crate::Layout::size_rules`].
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

    /// Construct from margins
    ///
    /// The `vertical` parameter determines which axis margins are read. The
    /// `num_inner ≥ 0` parameter controls how many additional rows or
    /// columns are added (beyond the first one).
    pub fn from_margins(margins: Margins, vertical: bool, num_inner: u32) -> Self {
        let a = if !vertical {
            margins.outer.0 + num_inner * margins.inner.0 as u32
        } else {
            margins.outer.1 + num_inner * margins.inner.1 as u32
        };
        SizeRules { a, b: a }
    }

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
