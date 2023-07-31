// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout visitor

// Methods have to take `&mut self`
#![allow(clippy::wrong_self_convention)]

use super::{Align, AlignHints, AlignPair, AxisInfo, SizeRules};
use super::{DynRowStorage, RowPositionSolver, RowSetter, RowSolver, RowStorage};
use super::{GridChildInfo, GridDimensions, GridSetter, GridSolver, GridStorage};
use super::{RulesSetter, RulesSolver};
use crate::draw::color::Rgb;
use crate::event::ConfigCx;
use crate::geom::{Coord, Offset, Rect, Size};
use crate::theme::{Background, DrawCx, FrameStyle, MarginStyle, SizeCx};
use crate::WidgetId;
use crate::{dir::Directional, dir::Directions, Layout};
use std::iter::ExactSizeIterator;

/// A sub-set of [`Layout`] used by [`Visitor`].
///
/// Unlike when implementing a widget, all methods of this trait must be
/// implemented directly.
pub trait Visitable {
    /// Get size rules for the given axis
    ///
    /// This method is identical to [`Layout::size_rules`].
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules;

    /// Set size and position
    ///
    /// This method is identical to [`Layout::set_rect`].
    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect);

    /// Translate a coordinate to a [`WidgetId`]
    ///
    /// Implementations should recursively call `find_id` on children, returning
    /// `None` if no child returns a `WidgetId`.
    /// This method is simplified relative to [`Layout::find_id`].
    fn find_id(&mut self, coord: Coord) -> Option<WidgetId>;

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
pub struct Visitor<'a> {
    layout: LayoutType<'a>,
}

/// Items which can be placed in a layout
enum LayoutType<'a> {
    /// A boxed component
    BoxComponent(Box<dyn Visitable + 'a>),
    /// A single child widget
    Single(&'a mut dyn Layout),
    /// A single child widget with alignment
    AlignSingle(&'a mut dyn Layout, AlignHints),
    /// Apply alignment hints to some sub-layout
    Align(Box<Visitor<'a>>, AlignHints),
    /// Apply alignment and pack some sub-layout
    Pack(Box<Visitor<'a>>, &'a mut PackStorage, AlignHints),
    /// Replace (some) margins
    Margins(Box<Visitor<'a>>, Directions, MarginStyle),
    /// Frame around content
    Frame(Box<Visitor<'a>>, &'a mut FrameStorage, FrameStyle),
    /// Button frame around content
    Button(Box<Visitor<'a>>, &'a mut FrameStorage, Option<Rgb>),
}

impl<'a> Visitor<'a> {
    /// Construct a single-item layout
    pub fn single(widget: &'a mut dyn Layout) -> Self {
        let layout = LayoutType::Single(widget);
        Visitor { layout }
    }

    /// Construct a single-item layout with alignment hints
    pub fn align_single(widget: &'a mut dyn Layout, hints: AlignHints) -> Self {
        let layout = LayoutType::AlignSingle(widget, hints);
        Visitor { layout }
    }

    /// Construct a sub-layout with alignment hints
    pub fn align(layout: Self, hints: AlignHints) -> Self {
        let layout = LayoutType::Align(Box::new(layout), hints);
        Visitor { layout }
    }

    /// Construct a sub-layout which is squashed and aligned
    pub fn pack(stor: &'a mut PackStorage, layout: Self, hints: AlignHints) -> Self {
        let layout = LayoutType::Pack(Box::new(layout), stor, hints);
        Visitor { layout }
    }

    /// Replace the margins of a sub-layout
    pub fn margins(layout: Self, dirs: Directions, margins: MarginStyle) -> Self {
        let layout = LayoutType::Margins(Box::new(layout), dirs, margins);
        Visitor { layout }
    }

    /// Construct a frame around a sub-layout
    ///
    /// This frame has dimensions according to [`SizeCx::frame`].
    pub fn frame(data: &'a mut FrameStorage, child: Self, style: FrameStyle) -> Self {
        let layout = LayoutType::Frame(Box::new(child), data, style);
        Visitor { layout }
    }

    /// Construct a button frame around a sub-layout
    ///
    /// Generates a button frame containing the child node. Mouse/touch input
    /// on the button reports input to `self`, not to the child node.
    pub fn button(data: &'a mut FrameStorage, child: Self, color: Option<Rgb>) -> Self {
        let layout = LayoutType::Button(Box::new(child), data, color);
        Visitor { layout }
    }

    /// Construct a row/column layout over an iterator of layouts
    pub fn list<I, D, S>(list: I, direction: D, data: &'a mut S) -> Self
    where
        I: ExactSizeIterator<Item = Visitor<'a>> + 'a,
        D: Directional,
        S: RowStorage,
    {
        let layout = LayoutType::BoxComponent(Box::new(List {
            children: list,
            direction,
            data,
        }));
        Visitor { layout }
    }

    /// Construct a row/column layout over a slice of widgets
    ///
    /// In contrast to [`Visitor::list`], `slice` can only be used over a slice
    /// of a single type of widget, enabling some optimisations: `O(log n)` for
    /// `draw` and `find_id`. Some other methods, however, remain `O(n)`, thus
    /// the optimisations are not (currently) so useful.
    pub fn slice<W, D>(slice: &'a mut [W], direction: D, data: &'a mut DynRowStorage) -> Self
    where
        W: Layout,
        D: Directional,
    {
        let layout = LayoutType::BoxComponent(Box::new(Slice {
            children: slice,
            direction,
            data,
        }));
        Visitor { layout }
    }

    /// Construct a grid layout over an iterator of `(cell, layout)` items
    pub fn grid<I, S>(iter: I, dim: GridDimensions, data: &'a mut S) -> Self
    where
        I: DoubleEndedIterator<Item = (GridChildInfo, Visitor<'a>)> + 'a,
        S: GridStorage,
    {
        let layout = LayoutType::BoxComponent(Box::new(Grid {
            data,
            dim,
            children: iter,
        }));
        Visitor { layout }
    }

    /// Construct a float of layouts
    ///
    /// This is a stack, but showing all items simultaneously.
    /// The first item is drawn on top and has first input priority.
    pub fn float<I>(list: I) -> Self
    where
        I: DoubleEndedIterator<Item = Visitor<'a>> + 'a,
    {
        let layout = LayoutType::BoxComponent(Box::new(Float { children: list }));
        Visitor { layout }
    }

    /// Get size rules for the given axis
    #[inline]
    pub fn size_rules(mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.size_rules_(sizer, axis)
    }
    fn size_rules_(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        match &mut self.layout {
            LayoutType::BoxComponent(component) => component.size_rules(sizer, axis),
            LayoutType::Single(child) => child.size_rules(sizer, axis),
            LayoutType::AlignSingle(child, hints) => {
                child.size_rules(sizer, axis.with_align_hints(*hints))
            }
            LayoutType::Align(layout, hints) => {
                layout.size_rules_(sizer, axis.with_align_hints(*hints))
            }
            LayoutType::Pack(layout, stor, hints) => {
                let rules = layout.size_rules_(sizer, stor.apply_align(axis, *hints));
                stor.size.set_component(axis, rules.ideal_size());
                rules
            }
            LayoutType::Margins(child, dirs, margins) => {
                let mut child_rules = child.size_rules_(sizer.re(), axis);
                if dirs.intersects(Directions::from(axis)) {
                    let mut rule_margins = child_rules.margins();
                    let margins = sizer.margins(*margins).extract(axis);
                    if dirs.intersects(Directions::LEFT | Directions::UP) {
                        rule_margins.0 = margins.0;
                    }
                    if dirs.intersects(Directions::RIGHT | Directions::DOWN) {
                        rule_margins.1 = margins.1;
                    }
                    child_rules.set_margins(rule_margins);
                }
                child_rules
            }
            LayoutType::Frame(child, storage, style) => {
                let child_rules = child.size_rules_(sizer.re(), storage.child_axis(axis));
                storage.size_rules(sizer, axis, child_rules, *style)
            }
            LayoutType::Button(child, storage, _) => {
                let child_rules = child.size_rules_(sizer.re(), storage.child_axis_centered(axis));
                storage.size_rules(sizer, axis, child_rules, FrameStyle::Button)
            }
        }
    }

    /// Apply a given `rect` to self
    #[inline]
    pub fn set_rect(mut self, cx: &mut ConfigCx, rect: Rect) {
        self.set_rect_(cx, rect);
    }
    fn set_rect_(&mut self, cx: &mut ConfigCx, rect: Rect) {
        match &mut self.layout {
            LayoutType::BoxComponent(layout) => layout.set_rect(cx, rect),
            LayoutType::Single(child) => child.set_rect(cx, rect),
            LayoutType::Align(layout, _) => layout.set_rect_(cx, rect),
            LayoutType::AlignSingle(child, _) => child.set_rect(cx, rect),
            LayoutType::Pack(layout, stor, _) => layout.set_rect_(cx, stor.aligned_rect(rect)),
            LayoutType::Margins(child, _, _) => child.set_rect_(cx, rect),
            LayoutType::Frame(child, storage, _) | LayoutType::Button(child, storage, _) => {
                storage.rect = rect;
                let child_rect = Rect {
                    pos: rect.pos + storage.offset,
                    size: rect.size - storage.size,
                };
                child.set_rect_(cx, child_rect);
            }
        }
    }

    /// Find a widget by coordinate
    ///
    /// Does not return the widget's own identifier. See example usage in
    /// [`Visitor::find_id`].
    #[inline]
    pub fn find_id(mut self, coord: Coord) -> Option<WidgetId> {
        self.find_id_(coord)
    }
    fn find_id_(&mut self, coord: Coord) -> Option<WidgetId> {
        match &mut self.layout {
            LayoutType::BoxComponent(layout) => layout.find_id(coord),
            LayoutType::Single(child) | LayoutType::AlignSingle(child, _) => child.find_id(coord),
            LayoutType::Align(layout, _) => layout.find_id_(coord),
            LayoutType::Pack(layout, _, _) => layout.find_id_(coord),
            LayoutType::Margins(layout, _, _) => layout.find_id_(coord),
            LayoutType::Frame(child, _, _) => child.find_id_(coord),
            // Buttons steal clicks, hence Button never returns ID of content
            LayoutType::Button(_, _, _) => None,
        }
    }

    /// Draw a widget's children
    #[inline]
    pub fn draw(mut self, draw: DrawCx) {
        self.draw_(draw);
    }
    fn draw_(&mut self, mut draw: DrawCx) {
        match &mut self.layout {
            LayoutType::BoxComponent(layout) => layout.draw(draw),
            LayoutType::Single(child) | LayoutType::AlignSingle(child, _) => draw.recurse(*child),
            LayoutType::Align(layout, _) => layout.draw_(draw),
            LayoutType::Pack(layout, _, _) => layout.draw_(draw),
            LayoutType::Margins(layout, _, _) => layout.draw_(draw),
            LayoutType::Frame(child, storage, style) => {
                draw.frame(storage.rect, *style, Background::Default);
                child.draw_(draw);
            }
            LayoutType::Button(child, storage, color) => {
                let bg = match color {
                    Some(rgb) => Background::Rgb(*rgb),
                    None => Background::Default,
                };
                draw.frame(storage.rect, FrameStyle::Button, bg);
                child.draw_(draw);
            }
        }
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
    I: ExactSizeIterator<Item = Visitor<'a>>,
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

    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
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
    I: DoubleEndedIterator<Item = Visitor<'a>>,
{
    children: I,
}

impl<'a, I> Visitable for Float<'a, I>
where
    I: DoubleEndedIterator<Item = Visitor<'a>>,
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

    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
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

/// A row/column over a slice
struct Slice<'a, W: Layout, D: Directional> {
    children: &'a mut [W],
    direction: D,
    data: &'a mut DynRowStorage,
}

impl<'a, W: Layout, D: Directional> Visitable for Slice<'a, W, D> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in self.children.iter_mut().enumerate() {
            solver.for_child(self.data, n, |axis| child.size_rules(sizer.re(), axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, self.data);

        for (n, child) in self.children.iter_mut().enumerate() {
            child.set_rect(cx, setter.child_rect(self.data, n));
        }
    }

    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        let solver = RowPositionSolver::new(self.direction);
        solver
            .find_child_mut(self.children, coord)
            .and_then(|child| child.find_id(coord))
    }

    fn draw(&mut self, mut draw: DrawCx) {
        let solver = RowPositionSolver::new(self.direction);
        solver.for_children(self.children, draw.get_clip_rect(), |w| draw.recurse(w));
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
    I: DoubleEndedIterator<Item = (GridChildInfo, Visitor<'a>)>,
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

    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
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
        axis.set_align(Some(Align::Center));
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
