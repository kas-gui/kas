// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout visitor

// Methods have to take `&mut self`
#![allow(clippy::wrong_self_convention)]

use super::{AlignHints, AxisInfo, RulesSetter, RulesSolver, SetRectMgr, SizeRules, Storage};
use super::{DynRowStorage, RowPositionSolver, RowSetter, RowSolver, RowStorage};
use super::{GridChildInfo, GridDimensions, GridSetter, GridSolver, GridStorage};
use crate::component::Component;
use crate::draw::color::Rgb;
use crate::geom::{Coord, Offset, Rect, Size};
use crate::theme::{Background, DrawMgr, FrameStyle, SizeMgr};
use crate::WidgetId;
use crate::{dir::Directional, Widget};
use std::any::Any;
use std::iter::ExactSizeIterator;

/// Chaining layout storage
///
/// We support embedded layouts within a single widget which means that we must
/// support storage for multiple layouts, though commonly zero or one layout is
/// used. We therefore use a simple linked list.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[derive(Debug, Default)]
pub struct StorageChain(Option<(Box<StorageChain>, Box<dyn Storage>)>);

impl StorageChain {
    /// Access layout storage
    ///
    /// This storage is allocated and initialised via `f()` on first access.
    ///
    /// Panics if the type `T` differs from the initial usage.
    pub fn storage<T: Storage, F: FnOnce() -> T>(&mut self, f: F) -> (&mut T, &mut StorageChain) {
        if let StorageChain(Some(ref mut b)) = self {
            let storage =
                b.1.downcast_mut()
                    .unwrap_or_else(|| panic!("StorageChain::storage::<T>(): incorrect type T"));
            return (storage, &mut b.0);
        }
        // TODO(rust#42877): store (StorageChain, dyn Storage) tuple in a single box
        let s = Box::new(StorageChain(None));
        let t: Box<dyn Storage> = Box::new(f());
        *self = StorageChain(Some((s, t)));
        match self {
            StorageChain(Some(b)) => (b.1.downcast_mut::<T>().unwrap(), &mut b.0),
            _ => unreachable!(),
        }
    }
}

/// A layout visitor
///
/// This constitutes a "visitor" which iterates over each child widget. Layout
/// algorithm details are implemented over this visitor.
pub struct Layout<'a> {
    layout: LayoutType<'a>,
}

/// Items which can be placed in a layout
enum LayoutType<'a> {
    /// No layout
    None,
    /// A component
    Component(&'a mut dyn Component),
    /// A boxed component
    BoxComponent(Box<dyn Component + 'a>),
    /// A single child widget
    Single(&'a mut dyn Widget),
    /// A single child widget with alignment
    AlignSingle(&'a mut dyn Widget, AlignHints),
    /// Apply alignment hints to some sub-layout
    AlignLayout(Box<Layout<'a>>, AlignHints),
    /// Frame around content
    Frame(Box<Layout<'a>>, &'a mut FrameStorage, FrameStyle),
    /// Button frame around content
    Button(Box<Layout<'a>>, &'a mut FrameStorage, Option<Rgb>),
}

impl<'a> Default for Layout<'a> {
    fn default() -> Self {
        Layout::none()
    }
}

impl<'a> Layout<'a> {
    /// Construct an empty layout
    pub fn none() -> Self {
        let layout = LayoutType::None;
        Layout { layout }
    }

    /// Construct a single-item layout
    pub fn single(widget: &'a mut dyn Widget) -> Self {
        let layout = LayoutType::Single(widget);
        Layout { layout }
    }

    /// Construct a single-item layout with alignment hints
    pub fn align_single(widget: &'a mut dyn Widget, hints: AlignHints) -> Self {
        let layout = LayoutType::AlignSingle(widget, hints);
        Layout { layout }
    }

    /// Align a sub-layout
    pub fn align(layout: Self, hints: AlignHints) -> Self {
        let layout = LayoutType::AlignLayout(Box::new(layout), hints);
        Layout { layout }
    }

    /// Construct a frame around a sub-layout
    ///
    /// This frame has dimensions according to [`SizeMgr::frame`].
    pub fn frame(data: &'a mut FrameStorage, child: Self, style: FrameStyle) -> Self {
        let layout = LayoutType::Frame(Box::new(child), data, style);
        Layout { layout }
    }

    /// Construct a button frame around a sub-layout
    ///
    /// Generates a button frame containing the child node. Mouse/touch input
    /// on the button reports input to `self`, not to the child node.
    pub fn button(data: &'a mut FrameStorage, child: Self, color: Option<Rgb>) -> Self {
        let layout = LayoutType::Button(Box::new(child), data, color);
        Layout { layout }
    }

    /// Place a component in the layout
    pub fn component(component: &'a mut dyn Component) -> Self {
        let layout = LayoutType::Component(component);
        Layout { layout }
    }

    /// Construct a row/column layout over an iterator of layouts
    pub fn list<I, D, S>(list: I, direction: D, data: &'a mut S) -> Self
    where
        I: ExactSizeIterator<Item = Layout<'a>> + 'a,
        D: Directional,
        S: RowStorage,
    {
        let layout = LayoutType::BoxComponent(Box::new(List {
            data,
            direction,
            children: list,
        }));
        Layout { layout }
    }

    /// Construct a row/column layout over a slice of widgets
    ///
    /// In contrast to [`Layout::list`], `slice` can only be used over a slice
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
        Layout { layout }
    }

    /// Construct a grid layout over an iterator of `(cell, layout)` items
    pub fn grid<I, S>(iter: I, dim: GridDimensions, data: &'a mut S) -> Self
    where
        I: Iterator<Item = (GridChildInfo, Layout<'a>)> + 'a,
        S: GridStorage,
    {
        let layout = LayoutType::BoxComponent(Box::new(Grid {
            data,
            dim,
            children: iter,
        }));
        Layout { layout }
    }

    /// Get size rules for the given axis
    #[inline]
    pub fn size_rules(mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        self.size_rules_(mgr, axis)
    }
    fn size_rules_(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        let frame = |mgr: SizeMgr, child: &mut Layout, storage: &mut FrameStorage, mut style| {
            let frame_rules = mgr.frame(style, axis);
            let child_rules = child.size_rules_(mgr, axis);
            if axis.is_horizontal() && style == FrameStyle::MenuEntry {
                style = FrameStyle::InnerMargin;
            }
            let (rules, offset, size) = match style {
                FrameStyle::InnerMargin | FrameStyle::EditBox => {
                    frame_rules.surround_with_margin(child_rules)
                }
                FrameStyle::NavFocus => frame_rules.surround_as_margin(child_rules),
                _ => frame_rules.surround_no_margin(child_rules),
            };
            storage.offset.set_component(axis, offset);
            storage.size.set_component(axis, size);
            rules
        };
        match &mut self.layout {
            LayoutType::None => SizeRules::EMPTY,
            LayoutType::Component(component) => component.size_rules(mgr, axis),
            LayoutType::BoxComponent(component) => component.size_rules(mgr, axis),
            LayoutType::Single(child) => child.size_rules(mgr, axis),
            LayoutType::AlignSingle(child, _) => child.size_rules(mgr, axis),
            LayoutType::AlignLayout(layout, _) => layout.size_rules_(mgr, axis),
            LayoutType::Frame(child, storage, style) => frame(mgr, child, storage, *style),
            LayoutType::Button(child, storage, _) => frame(mgr, child, storage, FrameStyle::Button),
        }
    }

    /// Apply a given `rect` to self
    #[inline]
    pub fn set_rect(mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
        self.set_rect_(mgr, rect, align);
    }
    fn set_rect_(&mut self, mgr: &mut SetRectMgr, mut rect: Rect, align: AlignHints) {
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
                layout.set_rect_(mgr, rect, align);
            }
            LayoutType::Frame(child, storage, _) | LayoutType::Button(child, storage, _) => {
                storage.rect = rect;
                rect.pos += storage.offset;
                rect.size -= storage.size;
                child.set_rect_(mgr, rect, align);
            }
        }
    }

    /// Find a widget by coordinate
    ///
    /// Does not return the widget's own identifier. See example usage in
    /// [`Layout::find_id`].
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

impl<'a, S: RowStorage, D: Directional, I> Component for List<'a, S, D, I>
where
    I: ExactSizeIterator<Item = Layout<'a>>,
{
    fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in (&mut self.children).enumerate() {
            solver.for_child(self.data, n, |axis| child.size_rules(mgr.re(), axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
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

/// A row/column over a slice
struct Slice<'a, W: Widget, D: Directional> {
    data: &'a mut DynRowStorage,
    direction: D,
    children: &'a mut [W],
}

impl<'a, W: Widget, D: Directional> Component for Slice<'a, W, D> {
    fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in self.children.iter_mut().enumerate() {
            solver.for_child(self.data, n, |axis| child.size_rules(mgr.re(), axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
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

impl<'a, S: GridStorage, I> Component for Grid<'a, S, I>
where
    I: Iterator<Item = (GridChildInfo, Layout<'a>)>,
{
    fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        let mut solver = GridSolver::<Vec<_>, Vec<_>, _>::new(axis, self.dim, self.data);
        for (info, child) in &mut self.children {
            solver.for_child(self.data, info, |axis| child.size_rules(mgr.re(), axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
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
    // NOTE: potentially rect is redundant (e.g. with widget's rect) but if we
    // want an alternative as a generic solution then all draw methods must
    // calculate and pass the child's rect, which is probably worse.
    rect: Rect,
}
impl Storage for FrameStorage {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
