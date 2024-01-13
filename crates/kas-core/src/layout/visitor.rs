// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout visitor

// Methods have to take `&mut self`
#![allow(clippy::wrong_self_convention)]

use super::{AlignHints, AlignPair, AxisInfo, SizeRules};
use super::{GridChildInfo, GridDimensions, GridSetter, GridSolver, GridStorage};
use super::{RowSetter, RowSolver, RowStorage};
use super::{RulesSetter, RulesSolver};
use crate::draw::color::Rgb;
use crate::event::ConfigCx;
use crate::geom::{Coord, Offset, Rect, Size};
use crate::theme::{Background, DrawCx, FrameStyle, MarginStyle, SizeCx};
use crate::Id;
use crate::{dir::Directional, dir::Directions, Layout};
use std::iter::ExactSizeIterator;

/// A sub-set of [`Layout`] used by [`Visitor`].
///
/// Unlike when implementing a widget, all methods of this trait must be
/// implemented directly.
#[crate::autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Visitable {
    /// Get size rules for the given axis
    ///
    /// This method is identical to [`Layout::size_rules`].
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules;

    /// Set size and position
    ///
    /// This method is identical to [`Layout::set_rect`].
    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect);

    /// Translate a coordinate to an [`Id`]
    ///
    /// Implementations should recursively call `find_id` on children, returning
    /// `None` if no child returns an `Id`.
    /// This method is simplified relative to [`Layout::find_id`].
    fn find_id(&mut self, coord: Coord) -> Option<Id>;

    /// Draw a widget and its children
    ///
    /// This method is identical to [`Layout::draw`].
    fn draw(&mut self, draw: DrawCx);
}

/// A layout visitor
///
/// This constitutes a "visitor" which iterates over each child widget. Layout
/// algorithm details are implemented over this visitor.
///
/// This is an internal API and may be subject to unexpected breaking changes.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub struct Visitor<V: Visitable>(V);

/// Visitor using a boxed dyn trait object
pub type BoxVisitor<'a> = Visitor<Box<dyn Visitable + 'a>>;

impl<'a, V: Visitable> Visitor<V>
where
    V: 'a,
{
    #[cfg(feature = "min_spec")]
    #[inline]
    pub default fn boxed(self) -> Visitor<Box<dyn Visitable + 'a>> {
        Visitor(Box::new(self.0))
    }
    #[cfg(not(feature = "min_spec"))]
    #[inline]
    pub fn boxed(self) -> Visitor<Box<dyn Visitable + 'a>> {
        Visitor(Box::new(self.0))
    }
}

impl<'a> BoxVisitor<'a> {
    #[cfg(feature = "min_spec")]
    #[inline]
    pub fn boxed(self) -> Visitor<Box<dyn Visitable + 'a>> {
        self
    }

    /// Construct a single-item layout
    pub fn single(widget: &'a mut dyn Layout) -> Visitor<impl Visitable + 'a> {
        Visitor(Single { widget })
    }

    /// Construct a single-item layout with alignment hints
    pub fn align_single(
        widget: &'a mut dyn Layout,
        hints: AlignHints,
    ) -> Visitor<impl Visitable + 'a> {
        Self::align(Self::single(widget), hints)
    }

    /// Construct a sub-layout with alignment hints
    pub fn align<C: Visitable + 'a>(child: C, hints: AlignHints) -> Visitor<impl Visitable + 'a> {
        Visitor(Align { child, hints })
    }

    /// Construct a sub-layout which is squashed and aligned
    pub fn pack<C: Visitable + 'a>(
        storage: &'a mut PackStorage,
        child: C,
        hints: AlignHints,
    ) -> Visitor<impl Visitable + 'a> {
        Visitor(Pack {
            child,
            storage,
            hints,
        })
    }

    /// Replace the margins of a sub-layout
    pub fn margins<C: Visitable + 'a>(
        child: C,
        dirs: Directions,
        style: MarginStyle,
    ) -> Visitor<impl Visitable + 'a> {
        Visitor(Margins { child, dirs, style })
    }

    /// Construct a frame around a sub-layout
    ///
    /// This frame has dimensions according to [`SizeCx::frame`].
    pub fn frame<C: Visitable + 'a>(
        storage: &'a mut FrameStorage,
        child: C,
        style: FrameStyle,
    ) -> Visitor<impl Visitable + 'a> {
        Visitor(Frame {
            child,
            storage,
            style,
        })
    }

    /// Construct a button frame around a sub-layout
    ///
    /// Generates a button frame containing the child node. Mouse/touch input
    /// on the button reports input to `self`, not to the child node.
    pub fn button<C: Visitable + 'a>(
        storage: &'a mut FrameStorage,
        child: C,
        color: Option<Rgb>,
    ) -> Visitor<impl Visitable + 'a> {
        Visitor(Button {
            child,
            storage,
            color,
        })
    }

    /// Construct a row/column layout over an iterator of layouts
    pub fn list<I, D, S>(list: I, direction: D, data: &'a mut S) -> Visitor<impl Visitable + 'a>
    where
        I: ExactSizeIterator<Item = BoxVisitor<'a>> + 'a,
        D: Directional,
        S: RowStorage,
    {
        Visitor(List {
            children: list,
            direction,
            data,
        })
    }

    /// Construct a float of layouts
    ///
    /// This is a stack, but showing all items simultaneously.
    /// The first item is drawn on top and has first input priority.
    pub fn float<I>(list: I) -> Visitor<impl Visitable + 'a>
    where
        I: DoubleEndedIterator<Item = BoxVisitor<'a>> + 'a,
    {
        Visitor(Float { children: list })
    }

    /// Construct a grid layout over an iterator of `(cell, layout)` items
    pub fn grid<I, S>(iter: I, dim: GridDimensions, data: &'a mut S) -> Visitor<impl Visitable + 'a>
    where
        I: DoubleEndedIterator<Item = (GridChildInfo, BoxVisitor<'a>)> + 'a,
        S: GridStorage,
    {
        Visitor(Grid {
            data,
            dim,
            children: iter,
        })
    }
}

impl<V: Visitable> Visitor<V> {
    /// Get size rules for the given axis
    #[inline]
    pub fn size_rules(mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.size_rules_(sizer, axis)
    }
    fn size_rules_(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.0.size_rules(sizer, axis)
    }

    /// Apply a given `rect` to self
    #[inline]
    pub fn set_rect(mut self, cx: &mut ConfigCx, rect: Rect) {
        self.set_rect_(cx, rect);
    }
    fn set_rect_(&mut self, cx: &mut ConfigCx, rect: Rect) {
        self.0.set_rect(cx, rect);
    }

    /// Find a widget by coordinate
    ///
    /// Does not return the widget's own identifier. See example usage in
    /// [`Visitor::find_id`].
    #[inline]
    pub fn find_id(mut self, coord: Coord) -> Option<Id> {
        self.find_id_(coord)
    }
    fn find_id_(&mut self, coord: Coord) -> Option<Id> {
        self.0.find_id(coord)
    }

    /// Draw a widget's children
    #[inline]
    pub fn draw(mut self, draw: DrawCx) {
        self.draw_(draw);
    }
    fn draw_(&mut self, draw: DrawCx) {
        self.0.draw(draw);
    }
}

impl<V: Visitable> Visitable for Visitor<V> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.size_rules_(sizer, axis)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        self.set_rect_(cx, rect);
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        self.find_id_(coord)
    }

    fn draw(&mut self, draw: DrawCx) {
        self.draw_(draw);
    }
}

struct Single<'a> {
    widget: &'a mut dyn Layout,
}

impl<'a> Visitable for Single<'a> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.widget.size_rules(sizer, axis)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        self.widget.set_rect(cx, rect);
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        self.widget.find_id(coord)
    }

    fn draw(&mut self, mut draw: DrawCx) {
        draw.recurse(self.widget)
    }
}

struct Align<C: Visitable> {
    child: C,
    hints: AlignHints,
}

impl<C: Visitable> Visitable for Align<C> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.child
            .size_rules(sizer, axis.with_align_hints(self.hints))
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        self.child.set_rect(cx, rect);
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        self.child.find_id(coord)
    }

    fn draw(&mut self, draw: DrawCx) {
        self.child.draw(draw);
    }
}

struct Pack<'a, C: Visitable> {
    child: C,
    storage: &'a mut PackStorage,
    hints: AlignHints,
}

impl<'a, C: Visitable> Visitable for Pack<'a, C> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let rules = self
            .child
            .size_rules(sizer, self.storage.apply_align(axis, self.hints));
        self.storage.size.set_component(axis, rules.ideal_size());
        rules
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        self.child.set_rect(cx, self.storage.aligned_rect(rect));
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        self.child.find_id(coord)
    }

    fn draw(&mut self, draw: DrawCx) {
        self.child.draw(draw);
    }
}

struct Margins<C: Visitable> {
    child: C,
    dirs: Directions,
    style: MarginStyle,
}

impl<C: Visitable> Visitable for Margins<C> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let mut child_rules = self.child.size_rules(sizer.re(), axis);
        if self.dirs.intersects(Directions::from(axis)) {
            let mut rule_margins = child_rules.margins();
            let margins = sizer.margins(self.style).extract(axis);
            if self.dirs.intersects(Directions::LEFT | Directions::UP) {
                rule_margins.0 = margins.0;
            }
            if self.dirs.intersects(Directions::RIGHT | Directions::DOWN) {
                rule_margins.1 = margins.1;
            }
            child_rules.set_margins(rule_margins);
        }
        child_rules
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        self.child.set_rect(cx, rect);
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        self.child.find_id(coord)
    }

    fn draw(&mut self, draw: DrawCx) {
        self.child.draw(draw);
    }
}

struct Frame<'a, C: Visitable> {
    child: C,
    storage: &'a mut FrameStorage,
    style: FrameStyle,
}

impl<'a, C: Visitable> Visitable for Frame<'a, C> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let child_rules = self
            .child
            .size_rules(sizer.re(), self.storage.child_axis(axis));
        self.storage
            .size_rules(sizer, axis, child_rules, self.style)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        self.storage.rect = rect;
        let child_rect = Rect {
            pos: rect.pos + self.storage.offset,
            size: rect.size - self.storage.size,
        };
        self.child.set_rect(cx, child_rect);
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        self.child.find_id(coord)
    }

    fn draw(&mut self, mut draw: DrawCx) {
        draw.frame(self.storage.rect, self.style, Background::Default);
        self.child.draw(draw);
    }
}

struct Button<'a, C: Visitable> {
    child: C,
    storage: &'a mut FrameStorage,
    color: Option<Rgb>,
}

impl<'a, C: Visitable> Visitable for Button<'a, C> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let child_rules = self
            .child
            .size_rules(sizer.re(), self.storage.child_axis_centered(axis));
        self.storage
            .size_rules(sizer, axis, child_rules, FrameStyle::Button)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        self.storage.rect = rect;
        let child_rect = Rect {
            pos: rect.pos + self.storage.offset,
            size: rect.size - self.storage.size,
        };
        self.child.set_rect(cx, child_rect);
    }

    fn find_id(&mut self, _: Coord) -> Option<Id> {
        // Buttons steal clicks, hence Button never returns ID of content
        None
    }

    fn draw(&mut self, mut draw: DrawCx) {
        let bg = match self.color {
            Some(rgb) => Background::Rgb(rgb),
            None => Background::Default,
        };
        draw.frame(self.storage.rect, FrameStyle::Button, bg);
        self.child.draw(draw);
    }
}

/// Implement row/column layout for children
struct List<'a, I, D, S> {
    children: I,
    direction: D,
    data: &'a mut S,
}

impl<'a, I, D: Directional, S: RowStorage> Visitable for List<'a, I, D, S>
where
    I: ExactSizeIterator<Item = BoxVisitor<'a>>,
{
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in (&mut self.children).enumerate() {
            solver.for_child(self.data, n, |axis| child.size_rules(sizer.re(), axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, self.data);

        for (n, child) in (&mut self.children).enumerate() {
            child.set_rect(cx, setter.child_rect(self.data, n));
        }
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        // TODO(opt): more efficient search strategy?
        self.children.find_map(|child| child.find_id(coord))
    }

    fn draw(&mut self, mut draw: DrawCx) {
        for child in &mut self.children {
            child.draw(draw.re_clone());
        }
    }
}

/// Float layout
struct Float<'a, I>
where
    I: DoubleEndedIterator<Item = BoxVisitor<'a>>,
{
    children: I,
}

impl<'a, I> Visitable for Float<'a, I>
where
    I: DoubleEndedIterator<Item = BoxVisitor<'a>>,
{
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let mut rules = SizeRules::EMPTY;
        for child in &mut self.children {
            rules = rules.max(child.size_rules(sizer.re(), axis));
        }
        rules
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        for child in &mut self.children {
            child.set_rect(cx, rect);
        }
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        self.children.find_map(|child| child.find_id(coord))
    }

    fn draw(&mut self, mut draw: DrawCx) {
        let mut iter = (&mut self.children).rev();
        if let Some(first) = iter.next() {
            first.draw(draw.re_clone());
        }
        for child in iter {
            draw.with_pass(|draw| child.draw(draw));
        }
    }
}

/// Implement grid layout for children
struct Grid<'a, S, I> {
    data: &'a mut S,
    dim: GridDimensions,
    children: I,
}

impl<'a, S: GridStorage, I> Visitable for Grid<'a, S, I>
where
    I: DoubleEndedIterator<Item = (GridChildInfo, BoxVisitor<'a>)>,
{
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let mut solver = GridSolver::<Vec<_>, Vec<_>, _>::new(axis, self.dim, self.data);
        for (info, child) in &mut self.children {
            solver.for_child(self.data, info, |axis| child.size_rules(sizer.re(), axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        let mut setter = GridSetter::<Vec<_>, Vec<_>, _>::new(rect, self.dim, self.data);
        for (info, child) in &mut self.children {
            child.set_rect(cx, setter.child_rect(self.data, info));
        }
    }

    fn find_id(&mut self, coord: Coord) -> Option<Id> {
        // TODO(opt): more efficient search strategy?
        self.children.find_map(|(_, child)| child.find_id(coord))
    }

    fn draw(&mut self, mut draw: DrawCx) {
        for (_, child) in (&mut self.children).rev() {
            child.draw(draw.re_clone());
        }
    }
}

/// Layout storage for alignment
#[derive(Clone, Default, Debug)]
pub struct PackStorage {
    align: AlignPair,
    size: Size,
}
impl PackStorage {
    /// Set alignment
    fn apply_align(&mut self, axis: AxisInfo, hints: AlignHints) -> AxisInfo {
        let axis = axis.with_align_hints(hints);
        self.align.set_component(axis, axis.align_or_default());
        axis
    }

    /// Align rect
    fn aligned_rect(&self, rect: Rect) -> Rect {
        self.align.aligned_rect(self.size, rect)
    }
}

/// Layout storage for frame layout
#[derive(Clone, Default, Debug)]
pub struct FrameStorage {
    /// Size used by frame (sum of widths of borders)
    pub size: Size,
    /// Offset of frame contents from parent position
    pub offset: Offset,
    // NOTE: potentially rect is redundant (e.g. with widget's rect) but if we
    // want an alternative as a generic solution then all draw methods must
    // calculate and pass the child's rect, which is probably worse.
    rect: Rect,
}
impl FrameStorage {
    /// Calculate child's "other axis" size
    pub fn child_axis(&self, mut axis: AxisInfo) -> AxisInfo {
        axis.sub_other(self.size.extract(axis.flipped()));
        axis
    }

    /// Calculate child's "other axis" size, forcing center-alignment of content
    pub fn child_axis_centered(&self, mut axis: AxisInfo) -> AxisInfo {
        axis.sub_other(self.size.extract(axis.flipped()));
        axis.set_align(Some(super::Align::Center));
        axis
    }

    /// Generate [`SizeRules`]
    pub fn size_rules(
        &mut self,
        sizer: SizeCx,
        axis: AxisInfo,
        child_rules: SizeRules,
        style: FrameStyle,
    ) -> SizeRules {
        let frame_rules = sizer.frame(style, axis);
        let (rules, offset, size) = frame_rules.surround(child_rules);
        self.offset.set_component(axis, offset);
        self.size.set_component(axis, size);
        rules
    }
}
