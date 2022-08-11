// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout visitor

// Methods have to take `&mut self`
#![allow(clippy::wrong_self_convention)]

use super::{Align, AlignHints, AxisInfo, MarginSelector, SizeRules};
use super::{DynRowStorage, RowPositionSolver, RowSetter, RowSolver, RowStorage};
use super::{GridChildInfo, GridDimensions, GridSetter, GridSolver, GridStorage};
use super::{RulesSetter, RulesSolver};
use crate::draw::color::Rgb;
use crate::event::ConfigMgr;
use crate::geom::{Coord, Offset, Rect, Size};
use crate::theme::{Background, DrawMgr, FrameStyle, SizeMgr};
use crate::WidgetId;
use crate::{dir::Directional, dir::Directions, Layout, Widget};
use std::iter::ExactSizeIterator;

/// A layout visitor
///
/// This constitutes a "visitor" which iterates over each child widget. Layout
/// algorithm details are implemented over this visitor.
///
/// TODO: consider removal. This is currently used to implement the
/// `layout = ..` property of `#[widget]`, but may not be the best approach.
pub struct Visitor<'a> {
    layout: LayoutType<'a>,
}

/// Items which can be placed in a layout
enum LayoutType<'a> {
    /// No layout
    None,
    /// A component
    Component(&'a mut dyn Layout),
    /// A boxed component
    BoxComponent(Box<dyn Layout + 'a>),
    /// A single child widget
    Single(&'a mut dyn Widget),
    /// A single child widget with alignment
    AlignSingle(&'a mut dyn Widget, AlignHints),
    /// Apply alignment hints to some sub-layout
    AlignLayout(Box<Visitor<'a>>, AlignHints),
    /// Replace (some) margins
    Margins(Box<Visitor<'a>>, Directions, MarginSelector),
    /// Frame around content
    Frame(Box<Visitor<'a>>, &'a mut FrameStorage, FrameStyle),
    /// Button frame around content
    Button(Box<Visitor<'a>>, &'a mut FrameStorage, Option<Rgb>),
}

impl<'a> Default for Visitor<'a> {
    fn default() -> Self {
        Visitor::none()
    }
}

impl<'a> Visitor<'a> {
    /// Construct an empty layout
    pub fn none() -> Self {
        let layout = LayoutType::None;
        Visitor { layout }
    }

    /// Construct a single-item layout
    pub fn single(widget: &'a mut dyn Widget) -> Self {
        let layout = LayoutType::Single(widget);
        Visitor { layout }
    }

    /// Construct a single-item layout with alignment hints
    pub fn align_single(widget: &'a mut dyn Widget, hints: AlignHints) -> Self {
        let layout = LayoutType::AlignSingle(widget, hints);
        Visitor { layout }
    }

    /// Align a sub-layout
    pub fn align(layout: Self, hints: AlignHints) -> Self {
        let layout = LayoutType::AlignLayout(Box::new(layout), hints);
        Visitor { layout }
    }

    /// Replace the margins of a sub-layout
    pub fn margins(layout: Self, dirs: Directions, margins: MarginSelector) -> Self {
        let layout = LayoutType::Margins(Box::new(layout), dirs, margins);
        Visitor { layout }
    }

    /// Construct a frame around a sub-layout
    ///
    /// This frame has dimensions according to [`SizeMgr::frame`].
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

    /// Place a component in the layout
    pub fn component(component: &'a mut dyn Layout) -> Self {
        let layout = LayoutType::Component(component);
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
            data,
            direction,
            children: list,
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
        W: Widget,
        D: Directional,
    {
        let layout = LayoutType::BoxComponent(Box::new(Slice {
            data,
            direction,
            children: slice,
        }));
        Visitor { layout }
    }

    /// Construct a grid layout over an iterator of `(cell, layout)` items
    pub fn grid<I, S>(iter: I, dim: GridDimensions, data: &'a mut S) -> Self
    where
        I: Iterator<Item = (GridChildInfo, Visitor<'a>)> + 'a,
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
    pub fn size_rules(mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        self.size_rules_(mgr, axis)
    }
    fn size_rules_(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        match &mut self.layout {
            LayoutType::None => SizeRules::EMPTY,
            LayoutType::Component(component) => component.size_rules(mgr, axis),
            LayoutType::BoxComponent(component) => component.size_rules(mgr, axis),
            LayoutType::Single(child) => child.size_rules(mgr, axis),
            LayoutType::AlignSingle(child, _) => child.size_rules(mgr, axis),
            LayoutType::AlignLayout(layout, _) => layout.size_rules_(mgr, axis),
            LayoutType::Margins(child, dirs, margins) => {
                let mut child_rules = child.size_rules_(mgr.re(), axis);
                if dirs.intersects(Directions::from(axis)) {
                    let mut rule_margins = child_rules.margins();
                    let margins = margins.select(mgr).extract(axis);
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
                let child_rules = child.size_rules_(mgr.re(), storage.child_axis(axis));
                storage.size_rules(mgr, axis, child_rules, *style)
            }
            LayoutType::Button(child, storage, _) => {
                let child_rules = child.size_rules_(mgr.re(), storage.child_axis(axis));
                storage.size_rules(mgr, axis, child_rules, FrameStyle::Button)
            }
        }
    }

    /// Apply a given `rect` to self
    ///
    /// Return the aligned rect.
    #[inline]
    pub fn set_rect(mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) -> Rect {
        self.set_rect_(mgr, rect, align)
    }
    fn set_rect_(&mut self, mgr: &mut ConfigMgr, mut rect: Rect, align: AlignHints) -> Rect {
        match &mut self.layout {
            LayoutType::None => (),
            LayoutType::Component(component) => component.set_rect(mgr, rect, align),
            LayoutType::BoxComponent(layout) => layout.set_rect(mgr, rect, align),
            LayoutType::Single(child) => child.set_rect(mgr, rect, align),
            LayoutType::AlignSingle(child, hints) => {
                let align = hints.combine(align);
                child.set_rect(mgr, rect, align);
            }
            LayoutType::AlignLayout(layout, hints) => {
                let align = hints.combine(align);
                return layout.set_rect_(mgr, rect, align);
            }
            LayoutType::Margins(child, _, _) => return child.set_rect_(mgr, rect, align),
            LayoutType::Frame(child, storage, _) => {
                storage.rect = rect;
                let child_rect = Rect {
                    pos: rect.pos + storage.offset,
                    size: rect.size - storage.size,
                };
                child.set_rect_(mgr, child_rect, align);
            }
            LayoutType::Button(child, storage, _) => {
                rect = align
                    .complete(Align::Stretch, Align::Stretch)
                    .aligned_rect(storage.ideal_size, rect);
                storage.rect = rect;
                let child_rect = Rect {
                    pos: rect.pos + storage.offset,
                    size: rect.size - storage.size,
                };
                child.set_rect_(mgr, child_rect, AlignHints::CENTER);
            }
        }
        rect
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
            LayoutType::None => None,
            LayoutType::Component(component) => component.find_id(coord),
            LayoutType::BoxComponent(layout) => layout.find_id(coord),
            LayoutType::Single(child) | LayoutType::AlignSingle(child, _) => child.find_id(coord),
            LayoutType::AlignLayout(layout, _) => layout.find_id_(coord),
            LayoutType::Margins(layout, _, _) => layout.find_id_(coord),
            LayoutType::Frame(child, _, _) => child.find_id_(coord),
            // Buttons steal clicks, hence Button never returns ID of content
            LayoutType::Button(_, _, _) => None,
        }
    }

    /// Draw a widget's children
    #[inline]
    pub fn draw(mut self, draw: DrawMgr) {
        self.draw_(draw);
    }
    fn draw_(&mut self, mut draw: DrawMgr) {
        match &mut self.layout {
            LayoutType::None => (),
            LayoutType::Component(component) => component.draw(draw),
            LayoutType::BoxComponent(layout) => layout.draw(draw),
            LayoutType::Single(child) | LayoutType::AlignSingle(child, _) => draw.recurse(*child),
            LayoutType::AlignLayout(layout, _) => layout.draw_(draw),
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
struct List<'a, S, D, I> {
    data: &'a mut S,
    direction: D,
    children: I,
}

impl<'a, S: RowStorage, D: Directional, I> Layout for List<'a, S, D, I>
where
    I: ExactSizeIterator<Item = Visitor<'a>>,
{
    fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in (&mut self.children).enumerate() {
            solver.for_child(self.data, n, |axis| child.size_rules(mgr.re(), axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, self.data);

        for (n, child) in (&mut self.children).enumerate() {
            child.set_rect(mgr, setter.child_rect(self.data, n), align);
        }
    }

    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        // TODO(opt): more efficient search strategy?
        self.children.find_map(|child| child.find_id(coord))
    }

    fn draw(&mut self, mut draw: DrawMgr) {
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

impl<'a, I> Layout for Float<'a, I>
where
    I: DoubleEndedIterator<Item = Visitor<'a>>,
{
    fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        let mut rules = SizeRules::EMPTY;
        for child in &mut self.children {
            rules = rules.max(child.size_rules(mgr.re(), axis));
        }
        rules
    }

    fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
        for child in &mut self.children {
            child.set_rect(mgr, rect, align);
        }
    }

    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        self.children.find_map(|child| child.find_id(coord))
    }

    fn draw(&mut self, mut draw: DrawMgr) {
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
struct Slice<'a, W: Widget, D: Directional> {
    data: &'a mut DynRowStorage,
    direction: D,
    children: &'a mut [W],
}

impl<'a, W: Widget, D: Directional> Layout for Slice<'a, W, D> {
    fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in self.children.iter_mut().enumerate() {
            solver.for_child(self.data, n, |axis| child.size_rules(mgr.re(), axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, self.data);

        for (n, child) in self.children.iter_mut().enumerate() {
            child.set_rect(mgr, setter.child_rect(self.data, n), align);
        }
    }

    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        let solver = RowPositionSolver::new(self.direction);
        solver
            .find_child_mut(self.children, coord)
            .and_then(|child| child.find_id(coord))
    }

    fn draw(&mut self, mut draw: DrawMgr) {
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

impl<'a, S: GridStorage, I> Layout for Grid<'a, S, I>
where
    I: Iterator<Item = (GridChildInfo, Visitor<'a>)>,
{
    fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        let mut solver = GridSolver::<Vec<_>, Vec<_>, _>::new(axis, self.dim, self.data);
        for (info, child) in &mut self.children {
            solver.for_child(self.data, info, |axis| child.size_rules(mgr.re(), axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
        let mut setter = GridSetter::<Vec<_>, Vec<_>, _>::new(rect, self.dim, align, self.data);
        for (info, child) in &mut self.children {
            child.set_rect(mgr, setter.child_rect(self.data, info), align);
        }
    }

    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        // TODO(opt): more efficient search strategy?
        self.children.find_map(|(_, child)| child.find_id(coord))
    }

    fn draw(&mut self, mut draw: DrawMgr) {
        for (_, child) in &mut self.children {
            child.draw(draw.re_clone());
        }
    }
}

/// Layout storage for frame layout
#[derive(Clone, Default, Debug)]
pub struct FrameStorage {
    /// Size used by frame (sum of widths of borders)
    pub size: Size,
    /// Offset of frame contents from parent position
    pub offset: Offset,
    ideal_size: Size,
    // NOTE: potentially rect is redundant (e.g. with widget's rect) but if we
    // want an alternative as a generic solution then all draw methods must
    // calculate and pass the child's rect, which is probably worse.
    rect: Rect,
}
impl FrameStorage {
    /// Calculate child's "other axis" size
    pub fn child_axis(&self, mut axis: AxisInfo) -> AxisInfo {
        if let Some(mut other) = axis.other() {
            other -= self.size.extract(axis.flipped());
            axis = AxisInfo::new(axis.is_vertical(), Some(other));
        }
        axis
    }

    /// Generate [`SizeRules`]
    pub fn size_rules(
        &mut self,
        mgr: SizeMgr,
        axis: AxisInfo,
        child_rules: SizeRules,
        mut style: FrameStyle,
    ) -> SizeRules {
        let frame_rules = mgr.frame(style, axis);
        if axis.is_horizontal() && style == FrameStyle::MenuEntry {
            style = FrameStyle::EditBox;
        }
        let (rules, offset, size) = match style {
            FrameStyle::EditBox => frame_rules.surround_with_margin(child_rules),
            FrameStyle::NavFocus => frame_rules.surround_as_margin(child_rules),
            _ => frame_rules.surround_no_margin(child_rules),
        };
        self.offset.set_component(axis, offset);
        self.size.set_component(axis, size);
        self.ideal_size.set_component(axis, rules.ideal_size());
        rules
    }
}
