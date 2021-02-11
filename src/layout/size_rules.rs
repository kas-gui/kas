// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`SizeRules`] type

use smallvec::SmallVec;
use std::fmt;
use std::iter::Sum;

use crate::conv::{Cast, Conv};
use crate::dir::Directional;
use crate::geom::Size;

// for doc use
#[allow(unused)]
use kas::draw::SizeHandle;

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
    pub const ZERO: Margins = Margins::splat(0);

    /// Margins with equal size on each edge.
    #[inline]
    pub const fn splat(size: u16) -> Self {
        Margins::hv_splat(size, size)
    }

    /// Margins via horizontal and vertical sizes
    #[inline]
    pub const fn hv(horiz: (u16, u16), vert: (u16, u16)) -> Self {
        Margins { horiz, vert }
    }

    /// Margins via horizontal and vertical sizes
    #[inline]
    pub const fn hv_splat(h: u16, v: u16) -> Self {
        Margins {
            horiz: (h, h),
            vert: (v, v),
        }
    }

    /// Sum of horizontal margins
    #[inline]
    pub fn sum_horiz(&self) -> i32 {
        i32::from(self.horiz.0) + i32::from(self.horiz.1)
    }

    /// Sum of vertical margins
    #[inline]
    pub fn sum_vert(&self) -> i32 {
        i32::from(self.vert.0) + i32::from(self.vert.1)
    }

    /// Pad a size with margins
    pub fn pad(self, size: Size) -> Size {
        Size::new(size.0 + self.sum_horiz(), size.1 + self.sum_vert())
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
    Maximize,
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
/// All units are in pixels. Sizes usually come directly from [`SizeHandle`]
/// or from a fixed quantity multiplied by [`SizeHandle::scale_factor`].
///
/// ### Sizes
///
/// The widget size model is simple: a rectangular box, plus a margin on each
/// side. Widget sizes are calculated from available space and the `SizeRules`;
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
/// [`kas::Layout::set_rect`] and [`kas::layout::AlignHints`].
///
/// [`Rect`]: kas::geom::Rect
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct SizeRules {
    // minimum good size
    a: i32,
    // ideal size; b >= a
    b: i32,
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
    /// Warning: appending another size to `EMPTY` *does* include margins
    /// even though `EMPTY` itself has zero size. However, `EMPTY` itself has
    /// zero-size margins, so this only affects appending an `EMPTY` with a
    /// non-empty `SizeRules`.
    pub const EMPTY: Self = SizeRules::empty(StretchPolicy::Fixed);

    /// Empty space with the given stretch policy
    ///
    /// See warning on [`SizeRules::EMPTY`].
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
    pub fn fixed(size: i32, margins: (u16, u16)) -> Self {
        debug_assert!(size >= 0);
        SizeRules {
            a: size,
            b: size,
            m: margins,
            stretch: StretchPolicy::Fixed,
        }
    }

    /// Construct fixed-size rules from given data
    #[inline]
    pub fn extract_fixed<D: Directional>(dir: D, size: Size, margin: Margins) -> Self {
        let size = size.extract(dir);
        let m = if dir.is_horizontal() {
            margin.horiz
        } else {
            margin.vert
        };
        SizeRules::fixed(size, m)
    }

    /// Construct with custom rules
    ///
    /// Region size should meet the given `min`-imum size and has a given
    /// `ideal` size, plus a given `stretch` policy.
    ///
    /// Expected: `ideal >= min` (if not, ideal is clamped to min).
    #[inline]
    pub fn new(min: i32, ideal: i32, margins: (u16, u16), stretch: StretchPolicy) -> Self {
        debug_assert!(0 <= min && 0 <= ideal);
        SizeRules {
            a: min,
            b: ideal.max(min),
            m: margins,
            stretch,
        }
    }

    /// Get the minimum size
    #[inline]
    pub fn min_size(self) -> i32 {
        self.a
    }

    /// Get the ideal size
    #[inline]
    pub fn ideal_size(self) -> i32 {
        self.b
    }

    /// Get the max size
    ///
    /// With most stretch policies, this returns `i32::MAX`, but with
    /// [`StretchPolicy::Fixed`], this is [`SizeRules::ideal_size`].
    #[inline]
    pub fn max_size(self) -> i32 {
        match self.stretch {
            StretchPolicy::Fixed => self.b,
            _ => i32::MAX,
        }
    }

    /// Get the `(pre, post)` margin sizes
    #[inline]
    pub fn margins(self) -> (u16, u16) {
        self.m
    }

    /// Get the `(pre, post)` margin sizes, cast to `i32`
    #[inline]
    pub fn margins_i32(self) -> (i32, i32) {
        (self.m.0.into(), self.m.1.into())
    }

    /// Get the stretch policy
    #[inline]
    pub fn stretch(self) -> StretchPolicy {
        self.stretch
    }

    /// Set the stretch policy
    #[inline]
    pub fn set_stretch(&mut self, stretch: StretchPolicy) {
        self.stretch = stretch;
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

    /// Multiply the `(min, ideal)` size, including internal margins
    ///
    /// E.g. given `margin = margins.0 + margins.1` and factors `(2, 5)`, the
    /// minimum size is set to `min * 2 + margin` and the ideal to
    /// `ideal * 5 + 4 * margin`.
    ///
    /// Panics if either factor is 0.
    pub fn multiply_with_margin(&mut self, min_factor: i32, ideal_factor: i32) {
        let margin = i32::from(self.m.0) + i32::from(self.m.1);
        assert!(min_factor > 0);
        assert!(ideal_factor > 0);
        self.a = min_factor * self.a + (min_factor - 1) * margin;
        self.b = ideal_factor * self.b + (ideal_factor - 1) * margin;
    }

    /// Append the rules for `rhs` to self
    ///
    /// This implies that `rhs` rules concern an element to the right of or
    /// below self. Note that order matters since margins may be combined.
    ///
    /// Note also that appending [`SizeRules::EMPTY`] does include interior
    /// margins (those between `EMPTY` and the other rules) within the result.
    pub fn append(&mut self, rhs: SizeRules) {
        let c: i32 = self.m.1.max(rhs.m.0).into();
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
        let c: i32 = self.m.1.max(rhs.m.0).into();
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
            ((self.m.0 + self.m.1).into(), frame.m)
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

    /// Return the result of appending all given ranges
    pub fn sum(range: &[SizeRules]) -> SizeRules {
        range.iter().sum()
    }

    /// Return the result of appending all given ranges (min only)
    ///
    /// This is a specialised version of sum: only the minimum is calculated
    pub fn min_sum(range: &[SizeRules]) -> SizeRules {
        if range.is_empty() {
            return SizeRules::EMPTY;
        }

        let mut rules = range[0];
        for r in &range[1..] {
            rules.a += i32::from(rules.m.1.max(r.m.0)) + r.a;
        }
        rules.b = rules.a;
        rules.m.1 = range[range.len() - 1].m.1;
        rules
    }

    /// Set self to `self - x + y`, clamped to 0 or greater
    ///
    /// This is a specialised operation to join two spans, subtracing the
    /// common overlap (`x`), thus margins are `self.m.0` and `y.m.1`.
    pub fn sub_add(&mut self, x: Self, y: Self) {
        self.a = (self.a - x.a + y.a).max(0);
        self.b = (self.b - x.b + y.b).max(0);
        self.m.1 = y.m.1;
        self.stretch = self.stretch.max(y.stretch);
    }

    /// Reduce the minimum size
    ///
    /// If `min` is greater than the current minimum size, this has no effect.
    #[inline]
    pub fn reduce_min_to(&mut self, min: i32) {
        self.a = self.a.min(min);
    }

    /// Solve a sequence of rules
    ///
    /// This is the same as [`SizeRules::solve_seq`] except that it is assumed
    /// the rules' sum is included as the last element of rules.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[inline]
    pub fn solve_seq_total(out: &mut [i32], rules: &[Self], target: i32) {
        let len = rules.len() - 1;
        let total = rules[len];
        let rules = &rules[0..len];
        debug_assert_eq!(
            SizeRules::sum(rules),
            total,
            "solve_seq_total: invalid input (missing configure or invalid usage?)"
        );
        Self::solve_seq_(out, rules, total, target);
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
    /// Input requirements: `rules.len() == out.len()`.
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
    pub fn solve_seq(out: &mut [i32], rules: &[Self], target: i32) {
        let total = SizeRules::sum(rules);
        Self::solve_seq_(out, rules, total, target);
    }

    fn solve_seq_(out: &mut [i32], rules: &[Self], total: Self, target: i32) {
        type Targets = SmallVec<[i32; 16]>;
        #[allow(non_snake_case)]
        let N = out.len();
        assert_eq!(rules.len(), N);
        if N == 0 {
            return;
        }
        debug_assert!(out.iter().all(|w| *w >= 0));

        if target > total.a {
            // All minimum sizes can be met.
            out[0] = out[0].max(rules[0].a);
            let mut margin_sum = 0;
            let mut sum = out[0];
            let mut dist_under_b = (rules[0].b - out[0]).max(0);
            let mut dist_over_b = (out[0] - rules[0].b).max(0);
            for i in 1..N {
                out[i] = out[i].max(rules[i].a);
                margin_sum += i32::from((rules[i - 1].m.1).max(rules[i].m.0));
                sum += out[i];
                dist_under_b += (rules[i].b - out[i]).max(0);
                dist_over_b += (out[i] - rules[i].b).max(0);
            }
            let target = target - margin_sum;

            if sum == target {
                return;
            } else if sum < target {
                fn increase_targets<F: Fn(usize) -> i32>(
                    out: &mut [i32],
                    targets: &mut Targets,
                    base: F,
                    mut avail: i32,
                ) {
                    // Calculate ceiling above which sizes will not be increased
                    let mut any_removed = true;
                    while any_removed {
                        any_removed = false;
                        let count = i32::conv(targets.len());
                        let ceil = (avail + count - 1) / count; // round up
                        let mut t = 0;
                        while t < targets.len() {
                            let i = usize::conv(targets[t]);
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
                    let count = i32::conv(targets.len());
                    let per_elt = avail / count;
                    let extra = usize::conv(avail - per_elt * count);
                    assert!(extra < targets.len());
                    for t in 0..extra {
                        let i = usize::conv(targets[t]);
                        out[i] = base(i) + per_elt + 1;
                    }
                    for t in extra..targets.len() {
                        let i = usize::conv(targets[t]);
                        out[i] = base(i) + per_elt;
                    }
                }

                if target - sum >= dist_under_b {
                    // We can increase all sizes to their ideal. Since this may
                    // not be enough, we also count the number with highest
                    // stretch factor and how far these are over their ideal.
                    sum = 0;
                    let highest_stretch = total.stretch;
                    let mut targets = Targets::new();
                    let mut over = 0;
                    for i in 0..N {
                        out[i] = out[i].max(rules[i].b);
                        sum += out[i];
                        if rules[i].stretch == highest_stretch {
                            over += out[i] - rules[i].b;
                            targets.push(i.cast());
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
                            targets.push(i.cast());
                        }
                    }

                    let avail = target - sum + over;
                    increase_targets(out, &mut targets, |i| rules[i].a, avail);
                    debug_assert_eq!(target, (0..N).fold(0, |x, i| x + out[i]));
                }
            } else {
                // sum > target: we need to decrease some sizes
                fn reduce_targets<F: Fn(usize) -> i32>(
                    out: &mut [i32],
                    targets: &mut Targets,
                    base: F,
                    mut avail: i32,
                ) {
                    // We can ignore everything below the floor
                    let mut any_removed = true;
                    while any_removed {
                        any_removed = false;
                        let floor = avail / i32::conv(targets.len());
                        let mut t = 0;
                        while t < targets.len() {
                            let i = usize::conv(targets[t]);
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
                    let floor = avail / i32::conv(targets.len());
                    let extra = usize::conv(avail) - usize::conv(floor) * targets.len();
                    assert!(extra < targets.len());
                    for t in 0..extra {
                        let i = usize::conv(targets[t]);
                        out[i] = base(i) + floor + 1;
                    }
                    for t in extra..targets.len() {
                        let i = usize::conv(targets[t]);
                        out[i] = base(i) + floor;
                    }
                }

                if dist_over_b > sum - target {
                    // we do not go below ideal, and will keep at least one above
                    // calculate distance over for each stretch policy
                    const MAX_POLICY: usize = StretchPolicy::Maximize as usize + 1;
                    let mut dists = [0; MAX_POLICY];
                    for i in 0..N {
                        dists[rules[i].stretch as usize] += (out[i] - rules[i].b).max(0);
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
                                targets.push(i.cast());
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
                            targets.push(i.cast());
                        }
                    }
                    if sum > target {
                        let avail = target + margin_sum - total.a;
                        reduce_targets(out, &mut targets, |i| rules[i].a, avail);
                    }
                    debug_assert_eq!(target, (0..N).fold(0, |x, i| x + out[i]));
                }
            }
        } else {
            // Below minimum size: in this case we can ignore prior contents
            // of `out`.We reduce the maximum allowed size to hit our target.
            let mut excess = total.a - target;

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

    /// Ensure at least one of `rules` has stretch policy at least as high as self
    ///
    /// The stretch policies are increased according to the heighest `scores`.
    /// Required: `rules.len() == scores.len()`.
    pub(crate) fn distribute_stretch_over_by(self, rules: &mut [Self], scores: &[u32]) {
        assert_eq!(rules.len(), scores.len());
        if rules.iter().any(|r| r.stretch >= self.stretch) {
            return;
        }

        let highest = scores.iter().cloned().max().unwrap_or(0);
        for i in 0..rules.len() {
            if scores[i] == highest {
                rules[i].stretch = self.stretch;
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

        let excess_a = (self.a - sum.a).max(0);
        let excess_b = (self.b - sum.b).max(0);
        if excess_a == 0 && excess_b == 0 {
            return;
        }

        let highest_stretch = sum.stretch;
        let count = i32::conv(
            (0..len)
                .filter(|i| rules[*i].stretch == highest_stretch)
                .count(),
        );
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
