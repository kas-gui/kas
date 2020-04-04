// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`SizeRules`] type

use smallvec::SmallVec;
use std::fmt;
use std::iter::Sum;

use crate::geom::Size;

// TODO: new Margin model
/// Margin sizes
///
/// Used by the layout system for margins around child widgets. Margins may be
/// drawn in and handle events like any other widget area.
#[derive(Copy, Clone, Debug, Default)]
pub struct Margins {
    /// Size of horizontal margins
    pub horiz: (u16, u16),
    /// Size of vertical margins
    pub vert: (u16, u16),
}

impl Margins {
    /// Zero-sized margins
    pub const ZERO: Margins = Margins::uniform(0);

    /// Margins with equal size on each edge.
    #[inline]
    pub const fn uniform(size: u16) -> Self {
        Margins::hv_uniform(size, size)
    }

    /// Margins via horizontal and vertical sizes
    #[inline]
    pub const fn hv_uniform(h: u16, v: u16) -> Self {
        Margins {
            horiz: (h, h),
            vert: (v, v),
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
#[derive(Copy, Clone, Default)]
pub struct SizeRules {
    // minimum good size
    a: u32,
    // ideal size; b >= a
    b: u32,
    // (pre, post) margins
    m: (u16, u16),
    stretch: StretchPolicy,
}

impl fmt::Debug for SizeRules {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SizeRules {{ a: {}, b: {}, m: ({}, {}), stretch: {:?} }}",
            self.a, self.b, self.m.0, self.m.1, self.stretch
        )
    }
}

impl SizeRules {
    /// Empty (zero size) widget
    ///
    /// Warning: appending or appending to `EMPTY` *does* add the usual margins
    /// even though `EMPTY` itself has zero size. However, `EMPTY` itself has
    /// zero-size margins, so this only affects appending an `EMPTY` with a
    /// non-empty `SizeRules`.
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
    pub fn extract_fixed(vertical: bool, size: Size, margin: Margins) -> Self {
        if !vertical {
            SizeRules {
                a: size.0,
                b: size.0,
                m: margin.horiz,
                stretch: StretchPolicy::Fixed,
            }
        } else {
            SizeRules {
                a: size.1,
                b: size.1,
                m: margin.vert,
                stretch: StretchPolicy::Fixed,
            }
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
    ///
    /// Note also that appending [`SizeRules::EMPTY`] does include interior
    /// margins (those between `EMPTY` and the other rules) within the result.
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
    ///
    /// Note also that appending [`SizeRules::EMPTY`] does include interior
    /// margins (those between `EMPTY` and the other rules) within the result.
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
    /// Given a sequence of width (or height) `rules` from children and a
    /// `target` size, find an appropriate size for each child.
    /// The method attempts to ensure that:
    ///
    /// -   All widths are at least their minimum size requirement
    /// -   All widths are at least their ideal size requirement, if this can be
    ///     met without decreasing any widths
    /// -   Excess space is divided evenly among members with the highest
    ///     stretch policy
    ///
    /// Input requirements: `rules.len() == out.len() + 1`, where the last value
    /// in `rules` is a summation over all prior rules.
    ///
    /// This method is idempotent: given satisfactory input widths, these will
    /// be preserved. Moreover, this method attempts to ensure that if target
    /// is increased, then decreased back to the previous value, this will
    /// revert to the previous solution. (The reverse may not hold if widths
    /// had previously been affected by a different agent.)
    ///
    /// This method's calculations are not affected by margins, except that it
    /// is assumed that the last entry of `rules` is a summation over all
    /// previous entries which does respect margins.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    pub fn solve_seq(out: &mut [u32], rules: &[Self], target: u32) {
        type Targets = SmallVec<[u32; 16]>;
        #[allow(non_snake_case)]
        let N = out.len();
        assert!(rules.len() == N + 1);
        if N == 0 {
            return;
        }

        if target > rules[N].a {
            // All minimum sizes can be met.
            out[0] = out[0].max(rules[0].a);
            let mut margin_sum = 0;
            let mut sum = out[0];
            let mut dist_under_b = rules[0].b.saturating_sub(out[0]);
            let mut dist_over_b = out[0].saturating_sub(rules[0].b);
            for i in 1..N {
                out[i] = out[i].max(rules[i].a);
                margin_sum += (rules[i - 1].m.1).max(rules[i].m.0) as u32;
                sum += out[i];
                dist_under_b += rules[i].b.saturating_sub(out[i]);
                dist_over_b += out[i].saturating_sub(rules[i].b);
            }
            let target = target - margin_sum;

            if sum == target {
                return;
            } else if sum < target {
                fn increase_targets<F: Fn(usize) -> u32>(
                    out: &mut [u32],
                    targets: &mut Targets,
                    base: F,
                    mut avail: u32,
                ) {
                    // Calculate ceiling above which sizes will not be increased
                    let mut any_removed = true;
                    while any_removed {
                        any_removed = false;
                        let count = targets.len() as u32;
                        let ceil = (avail + count - 1) / count; // round up
                        let mut t = 0;
                        while t < targets.len() {
                            let i = targets[t] as usize;
                            if out[i] >= base(i) + ceil {
                                avail -= out[i] - base(i);
                                targets.remove(t);
                                any_removed = true;
                                continue;
                            }
                            t += 1;
                        }
                        if targets.is_empty() {
                            return;
                        }
                    }

                    // Since no more are removed by a ceiling, all remaining
                    // targets will be (approx) equal. Arbitrarily distribute
                    // rounding errors to the first ones.
                    let count = targets.len() as u32;
                    let per_elt = avail / count;
                    let extra = (avail - per_elt * count) as usize;
                    assert!(extra < targets.len());
                    for t in 0..extra {
                        let i = targets[t] as usize;
                        out[i] = base(i) + per_elt + 1;
                    }
                    for t in extra..targets.len() {
                        let i = targets[t] as usize;
                        out[i] = base(i) + per_elt;
                    }
                }

                if target - sum >= dist_under_b {
                    // We can increase all sizes to their ideal. Since this may
                    // not be enough, we also count the number with highest
                    // stretch factor and how far these are over their ideal.
                    sum = 0;
                    let highest_stretch = rules[N].stretch;
                    let mut targets = Targets::new();
                    let mut over = 0;
                    for i in 0..N {
                        out[i] = out[i].max(rules[i].b);
                        sum += out[i];
                        if rules[i].stretch == highest_stretch {
                            over += out[i] - rules[i].b;
                            targets.push(i as u32);
                        }
                    }

                    let avail = target - sum + over;
                    increase_targets(out, &mut targets, |i| rules[i].b, avail);
                    debug_assert_eq!(target, (0..N).fold(0, |x, i| x + out[i]));
                } else {
                    // We cannot increase sizes as far as their ideal: instead
                    // increase over minimum size and under ideal
                    let mut targets = Targets::new();
                    let mut over = 0;
                    for i in 0..N {
                        if out[i] < rules[i].b {
                            over += out[i] - rules[i].a;
                            targets.push(i as u32);
                        }
                    }

                    let avail = target - sum + over;
                    increase_targets(out, &mut targets, |i| rules[i].a, avail);
                    debug_assert_eq!(target, (0..N).fold(0, |x, i| x + out[i]));
                }
            } else {
                // sum > target: we need to decrease some sizes
                fn reduce_targets<F: Fn(usize) -> u32>(
                    out: &mut [u32],
                    targets: &mut Targets,
                    base: F,
                    mut avail: u32,
                ) {
                    // We can ignore everything below the floor
                    let mut any_removed = true;
                    while any_removed {
                        any_removed = false;
                        let floor = avail / targets.len() as u32;
                        let mut t = 0;
                        while t < targets.len() {
                            let i = targets[t] as usize;
                            if out[i] <= base(i) + floor {
                                avail -= out[i] - base(i);
                                targets.remove(t);
                                any_removed = true;
                                continue;
                            }
                            t += 1;
                        }
                    }

                    // All targets remaining must be reduced to floor, bar rounding errors
                    let floor = avail / targets.len() as u32;
                    let extra = avail as usize - floor as usize * targets.len();
                    assert!(extra < targets.len());
                    for t in 0..extra {
                        let i = targets[t] as usize;
                        out[i] = base(i) + floor + 1;
                    }
                    for t in extra..targets.len() {
                        let i = targets[t] as usize;
                        out[i] = base(i) + floor;
                    }
                }

                if dist_over_b > sum - target {
                    // we do not go below ideal, and will keep at least one above
                    // calculate distance over for each stretch policy
                    const MAX_POLICY: usize = StretchPolicy::Maximise as usize + 1;
                    let mut dists = [0; MAX_POLICY];
                    for i in 0..N {
                        dists[rules[i].stretch as usize] += out[i].saturating_sub(rules[i].b);
                    }
                    let mut accum = 0;
                    let mut highest_affected = 0;
                    for i in 0..MAX_POLICY {
                        highest_affected = i;
                        dists[i] += accum;
                        accum = dists[i];
                        if accum >= sum - target {
                            break;
                        }
                    }

                    let mut avail = 0;
                    let mut targets = Targets::new();
                    for i in 0..N {
                        let stretch = rules[i].stretch as usize;
                        if out[i] > rules[i].b {
                            if stretch < highest_affected {
                                sum -= out[i] - rules[i].b;
                                out[i] = rules[i].b;
                            } else if stretch == highest_affected {
                                avail += out[i] - rules[i].b;
                                targets.push(i as u32);
                            }
                        }
                    }
                    if sum > target {
                        avail = avail + target - sum;
                        reduce_targets(out, &mut targets, |i| rules[i].b, avail);
                    }
                    debug_assert_eq!(target, (0..N).fold(0, |x, i| x + out[i]));
                } else {
                    // No size can exceed the ideal
                    // First, ensure nothing exceeds the ideal:
                    let mut targets = Targets::new();
                    sum = 0;
                    for i in 0..N {
                        out[i] = out[i].min(rules[i].b);
                        sum += out[i];
                        if out[i] > rules[i].a {
                            targets.push(i as u32);
                        }
                    }
                    if sum > target {
                        let avail = target + margin_sum - rules[N].a;
                        reduce_targets(out, &mut targets, |i| rules[i].a, avail);
                    }
                    debug_assert_eq!(target, (0..N).fold(0, |x, i| x + out[i]));
                }
            }
        } else {
            // Below minimum size: in this case we can ignore prior contents
            // of `out`.We reduce the maximum allowed size to hit our target.
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
    fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        if let Some(first) = iter.next() {
            iter.fold(first, |x, y| x.appended(y))
        } else {
            SizeRules::EMPTY
        }
    }
}

/// Return the sum over a sequence of rules, assuming these are ordered
///
/// Uses [`SizeRules::appended`] on all rules in sequence.
impl<'a> Sum<&'a Self> for SizeRules {
    fn sum<I: Iterator<Item = &'a Self>>(mut iter: I) -> Self {
        if let Some(first) = iter.next() {
            iter.fold(*first, |x, y| x.appended(*y))
        } else {
            SizeRules::EMPTY
        }
    }
}
