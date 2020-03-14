// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`SizeRules`] type

use std::iter::Sum;

use crate::geom::Size;
use crate::Direction;

// TODO: new Margin model
/// Margin sizes
///
/// Used by the layout system for margins around child widgets. Margins may be
/// drawn in and handle events like any other widget area.
#[derive(Copy, Clone, Debug, Default)]
pub struct Margins {
    /// Size of top/left margin
    pub first: (u16, u16),
    /// Size of bottom/right margin
    pub last: (u16, u16),
}

impl Margins {
    /// Zero-sized margins
    pub const ZERO: Margins = Margins::uniform(0);

    /// Margins with equal size on each edge.
    #[inline]
    pub const fn uniform(size: u16) -> Self {
        Margins::hv(size, size)
    }

    /// Margins via horizontal and vertical sizes
    #[inline]
    pub const fn hv(h: u16, v: u16) -> Self {
        Margins {
            first: (h, v),
            last: (h, v),
        }
    }
}

/// Policy for stretching widgets beyond ideal size
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum StretchPolicy {
    /// Do not exceed ideal size
    Fixed,
    /// Can be stretched to fill space but without utility
    Filler,
    /// Extra space has low utility
    LowUtility,
    /// Extra space has high utility
    HighUtility,
    /// Greedily consume as much space as possible
    Maximise,
}

impl Default for StretchPolicy {
    fn default() -> Self {
        StretchPolicy::Fixed
    }
}

/// Widget sizing information
///
/// This is the return value of [`kas::Layout::size_rules`] and is used to
/// describe size and margin requirements for widgets. This type only concerns
/// size requirements along a *single* axis.
///
/// All units are in pixels. Widgets should not hard-code size values but should
/// get their sizes from the [`kas::draw::SizeHandle`] trait, which handles
/// scaling for DPI factor.
///
/// ### Sizes
///
/// The widget size model is simple: a rectangular box, plus a margin on each
/// side.Widget sizes are calculated from available space and the `SizeRules`;
/// these rules currently include:
///
/// - the minimum size required for correct operation
/// - the preferred / ideal size
/// - a [`StretchPolicy`]
///
/// Available space is distributed between widgets depending on whether the
/// space is below the minimum, between the minimum and preferred, or above
/// the preferred size, with widgets with the highest [`StretchPolicy`] being
/// prioritised extra space. Usually rows/columns will be stretched to use all
/// available space, the exception being when none have a policy higher than
/// [`StretchPolicy::Fixed`]. When expanding a row/column, the highest stretch
/// policy of all contents will be used.
///
/// ### Margins
///
/// Required margin sizes are handled separately for each side of a widget.
/// Since [`SizeRules`] concerns only one axis, it stores only two margin sizes:
/// "pre" (left/top) and "post" (right/bottom). These are stored as `u16` values
/// on the assumption that no margin need exceed 65536.
///
/// When widgets are placed next to each other, their margins may be combined;
/// e.g. if a widget with margin of 6px is followed by another with margin 2px,
/// the required margin between the two is the maximum, 6px.
///
/// Only the layout engine and parent widgets need consider margins (beyond
/// their specification). For these cases, one needs to be aware that due to
/// margin-merging behaviour, one cannot simply "add" two `SizeRules`. Instead,
/// when placing one widget next to another, use [`SizeRules::append`] or
/// [`SizeRules::appended`]; when placing a widget within a frame, use
/// [`SizeRules::surrounded_by`]. When calculating the size of a sequence of
/// widgets, one may use the [`Sum`] implementation (this assumes that the
/// sequence is in left-to-right or top-to-bottom order).
///
/// ### Alignment
///
/// `SizeRules` concerns calculations of size requirements, which the layout
/// engine uses to assign each widget a [`Rect`]; it is up to the widget itself
/// to either fill this rect or align itself within the given space.
/// See [`kas::Layout::set_rect`] for more information.
///
/// For widgets with a stretch policy of [`StretchPolicy::Fixed`], it is still
/// possible for layout code to assign a size larger than the preference. It is
/// up to the widget to align itself within this space: see
/// [`kas::Layout::set_rect`] and [`kas::AlignHints`].
///
/// [`Rect`]: kas::geom::Rect
#[derive(Copy, Clone, Debug, Default)]
pub struct SizeRules {
    // minimum good size
    a: u32,
    // ideal size; b >= a
    b: u32,
    // (pre, post) margins
    m: (u16, u16),
    stretch: StretchPolicy,
}

impl SizeRules {
    /// Empty (zero size)
    pub const EMPTY: Self = SizeRules::empty(StretchPolicy::Fixed);

    /// Empty space with the given stretch policy
    #[inline]
    pub const fn empty(stretch: StretchPolicy) -> Self {
        SizeRules {
            a: 0,
            b: 0,
            m: (0, 0),
            stretch,
        }
    }

    /// A fixed size with given `(pre, post)` margins
    #[inline]
    pub fn fixed(size: u32, margins: (u16, u16)) -> Self {
        SizeRules {
            a: size,
            b: size,
            m: margins,
            stretch: StretchPolicy::Fixed,
        }
    }

    /// Construct fixed-size rules from given data
    #[inline]
    pub fn extract_fixed(dir: Direction, size: Size, margin: Margins) -> Self {
        match dir {
            Direction::Horizontal => SizeRules {
                a: size.0,
                b: size.0,
                m: (margin.first.0, margin.last.0),
                stretch: StretchPolicy::Fixed,
            },
            Direction::Vertical => SizeRules {
                a: size.1,
                b: size.1,
                m: (margin.first.1, margin.last.1),
                stretch: StretchPolicy::Fixed,
            },
        }
    }

    /// Construct with custom rules
    ///
    /// Region size should meet the given `min`-imum size and has a given
    /// `ideal` size, plus a given `stretch` policy.
    ///
    /// Required: `ideal >= min` (if not, ideal is clamped to min).
    #[inline]
    pub fn new(min: u32, ideal: u32, margins: (u16, u16), stretch: StretchPolicy) -> Self {
        SizeRules {
            a: min,
            b: ideal.max(min),
            m: margins,
            stretch,
        }
    }

    /// Get the minimum size
    #[inline]
    pub fn min_size(self) -> u32 {
        self.a
    }

    /// Get the ideal size
    #[inline]
    pub fn ideal_size(self) -> u32 {
        self.b
    }

    /// Get the `(pre, post)` margin sizes
    #[inline]
    pub fn margins(self) -> (u16, u16) {
        self.m
    }

    /// Set margins to max of own margins and given margins
    pub fn include_margins(&mut self, margins: (u16, u16)) {
        self.m.0 = self.m.0.max(margins.0);
        self.m.1 = self.m.1.max(margins.1);
    }

    /// Use the maximum size of `self` and `rhs`.
    #[inline]
    pub fn max(self, rhs: Self) -> SizeRules {
        SizeRules {
            a: self.a.max(rhs.a),
            b: self.b.max(rhs.b),
            m: (self.m.0.max(rhs.m.0), self.m.1.max(rhs.m.1)),
            stretch: self.stretch.max(rhs.stretch),
        }
    }

    /// Set `self = self.max(rhs);`
    #[inline]
    pub fn max_with(&mut self, rhs: Self) {
        *self = self.max(rhs);
    }

    /// Append the rules for `rhs` to self
    ///
    /// This implies that `rhs` rules concern an element to the right of or
    /// below self. Note that order matters since margins may be combined.
    pub fn append(&mut self, rhs: SizeRules) {
        let c = self.m.1.max(rhs.m.0) as u32;
        self.a += rhs.a + c;
        self.b += rhs.b + c;
        self.m.1 = rhs.m.1;
        self.stretch = self.stretch.max(rhs.stretch);
    }

    /// Return the rules for self appended by `rhs`
    ///
    ///
    /// This implies that `rhs` rules concern an element to the right of or
    /// below self. Note that order matters since margins may be combined.
    #[inline]
    pub fn appended(self, rhs: SizeRules) -> Self {
        let c = self.m.1.max(rhs.m.0) as u32;
        SizeRules {
            a: self.a + rhs.a + c,
            b: self.b + rhs.b + c,
            m: (self.m.0, rhs.m.1),
            stretch: self.stretch.max(rhs.stretch),
        }
    }

    /// Return the rules for self surrounded by `frame`
    ///
    /// If `internal_margins` are true, then space is allocated for `self`'s
    /// margins inside the frame; if not, then `self`'s margins are merged with
    /// the frame's margins.
    pub fn surrounded_by(self, frame: SizeRules, internal_margins: bool) -> Self {
        let (c, m) = if internal_margins {
            ((self.m.0 + self.m.1) as u32, frame.m)
        } else {
            (0, (self.m.0.max(frame.m.0), self.m.1.max(frame.m.1)))
        };
        SizeRules {
            a: self.a + frame.a + c,
            b: self.b + frame.b + c,
            m,
            stretch: self.stretch.max(frame.stretch),
        }
    }

    /// Set self to `self - x + y`
    pub fn sub_add(&mut self, x: Self, y: Self) {
        self.a = self.a + y.a - x.a;
        self.b = self.b + y.b - x.b;
        self.m.1 = y.m.1;
        self.stretch = self.stretch.max(y.stretch);
    }

    /// Reduce the minimum size
    ///
    /// If `min` is greater than the current minimum size, this has no effect.
    #[inline]
    pub fn reduce_min_to(&mut self, min: u32) {
        self.a = self.a.min(min);
    }

    /// Solve a sequence of rules
    ///
    /// Given a sequence of width / height `rules` from children (including a
    /// final value which is the total) and a `target` size, find an appropriate
    /// size and position for each child width / height.
    ///
    /// This method's calculations are not affected by margins, except that it
    /// is assumed that the last entry of `rules` is a summation over all
    /// previous entries which does respect margins.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    // TODO (const generics):
    // fn solve_seq<const N: usize>(out: &mut [u32; N], rules: &[Self; N + 1], target: u32)
    pub fn solve_seq(out: &mut [u32], rules: &[Self], target: u32) {
        #[allow(non_snake_case)]
        let N = out.len();
        assert!(rules.len() == N + 1);
        if N == 0 {
            return;
        }

        if target > rules[N].b {
            // Over the ideal size
            for i in 0..N {
                out[i] = rules[i].b;
            }

            let highest_stretch = rules[N].stretch;
            let count = (0..N)
                .filter(|i| rules[*i].stretch == highest_stretch)
                .count() as u32;
            let excess = target - rules[N].b;
            let per_elt = excess / count;
            let mut extra = excess - count * per_elt;
            for i in 0..N {
                if rules[i].stretch == highest_stretch {
                    out[i] += per_elt;
                    if extra > 0 {
                        out[i] += 1;
                        extra -= 1;
                    }
                }
            }
        } else if target >= rules[N].a {
            // At or over minimum: distribute extra relative to preferences.
            // TODO: perhaps this should not use the minimum except as a minimum?

            let mut excess = target - rules[N].a;
            let pref_excess = rules[N].b - rules[N].a;

            if pref_excess > 0 {
                let x = excess as f64 / pref_excess as f64;

                for n in 0..N {
                    // This will round down:
                    let r = rules[n];
                    let size = r.a + (x * (r.b - r.a) as f64) as u32;
                    out[n] = size;
                    excess -= size - r.a;
                }
            } else {
                // special case: pref_excess == 0
                let add = excess / N as u32;
                for n in 0..N {
                    let r = rules[n];
                    let size = r.a + add;
                    out[n] = size;
                    excess -= size - r.a;
                }
            }

            // The above may round down, which may leave us a little short.
            assert!(excess as usize <= N);
            // Distribute to first rem. sizes.
            for n in 0..(excess as usize) {
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

    /// Adjust a sequence of `rules` to ensure that the total is at least `self`
    ///
    /// This is used by grids to ensure that cell spans are sufficiently large.
    pub fn distribute_span_over(self, rules: &mut [Self]) {
        let len = rules.len();
        assert!(len > 0);
        let len1 = len - 1;
        let sum: SizeRules = rules.iter().sum();

        rules[0].m.0 = rules[0].m.0.max(self.m.0);
        rules[len1].m.1 = rules[len1].m.1.max(self.m.1);

        let excess_a = self.a.saturating_sub(sum.a);
        let excess_b = self.b.saturating_sub(sum.b);
        if excess_a == 0 && excess_b == 0 {
            return;
        }

        let highest_stretch = sum.stretch;
        let count = (0..len)
            .filter(|i| rules[*i].stretch == highest_stretch)
            .count() as u32;
        let a_per_elt = excess_a / count;
        let b_per_elt = excess_b / count;
        let mut extra_a = excess_a - count * a_per_elt;
        let mut extra_b = excess_b - count * b_per_elt;
        for i in 0..len {
            if rules[i].stretch == highest_stretch {
                rules[i].a += a_per_elt;
                rules[i].b += b_per_elt;
                if extra_a > 0 {
                    rules[i].a += 1;
                    extra_a -= 1;
                }
                if extra_b > 0 {
                    rules[i].b += 1;
                    extra_b -= 1;
                }
                if highest_stretch < self.stretch {
                    rules[i].stretch = self.stretch;
                }
            }
        }
    }
}

/// Return the sum over a sequence of rules, assuming these are ordered
///
/// Uses [`SizeRules::appended`] on all rules in sequence.
impl Sum for SizeRules {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(SizeRules::EMPTY, |x, y| x.appended(y))
    }
}

/// Return the sum over a sequence of rules, assuming these are ordered
///
/// Uses [`SizeRules::appended`] on all rules in sequence.
impl<'a> Sum<&'a Self> for SizeRules {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(SizeRules::EMPTY, |x, y| x.appended(*y))
    }
}
