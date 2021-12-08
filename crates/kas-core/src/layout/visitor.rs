// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout visitor

use super::{AlignHints, AxisInfo, RulesSetter, RulesSolver, SizeRules, Storage};
use super::{DynRowStorage, RowPositionSolver, RowSetter, RowSolver, RowStorage};
use super::{GridChildInfo, GridDimensions, GridSetter, GridSolver, GridStorage};
use crate::draw::{color::Rgb, DrawHandle, InputState, SizeHandle, TextClass};
use crate::event::{Manager, ManagerState};
use crate::geom::{Coord, Offset, Rect, Size};
use crate::text::{Align, TextApi, TextApiExt};
use crate::{dir::Directional, WidgetConfig};
use std::any::Any;
use std::iter::ExactSizeIterator;

/// Chaining layout storage
///
/// We support embedded layouts within a single widget which means that we must
/// support storage for multiple layouts, though commonly zero or one layout is
/// used. We therefore use a simple linked list.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[derive(Debug)]
pub struct StorageChain(Option<(Box<StorageChain>, Box<dyn Storage>)>);

impl Default for StorageChain {
    fn default() -> Self {
        StorageChain(None)
    }
}

impl StorageChain {
    /// Access layout storage
    ///
    /// This storage is allocated and initialised on first access.
    ///
    /// Panics if the type `T` differs from the initial usage.
    pub fn storage<T: Storage + Default>(&mut self) -> (&mut T, &mut StorageChain) {
        if let StorageChain(Some(ref mut b)) = self {
            let storage =
                b.1.downcast_mut()
                    .unwrap_or_else(|| panic!("StorageChain::storage::<T>(): incorrect type T"));
            return (storage, &mut b.0);
        }
        // TODO(rust#42877): store (StorageChain, dyn Storage) tuple in a single box
        let s = Box::new(StorageChain(None));
        let t: Box<dyn Storage> = Box::new(T::default());
        *self = StorageChain(Some((s, t)));
        match self {
            StorageChain(Some(b)) => (b.1.downcast_mut::<T>().unwrap(), &mut b.0),
            _ => unreachable!(),
        }
    }
}

/// Implementation helper for layout of children
trait Visitor {
    /// Get size rules for the given axis
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules;

    /// Apply a given `rect` to self
    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints);

    fn is_reversed(&mut self) -> bool;

    fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, state: InputState);
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
    /// A single child widget
    Single(&'a mut dyn WidgetConfig),
    /// A single child widget with alignment
    AlignSingle(&'a mut dyn WidgetConfig, AlignHints),
    /// Apply alignment hints to some sub-layout
    AlignLayout(Box<Layout<'a>>, AlignHints),
    /// Frame around content
    Frame(Box<Layout<'a>>, &'a mut FrameStorage),
    /// Navigation frame around content
    NavFrame(Box<Layout<'a>>, &'a mut FrameStorage),
    /// Button frame around content
    Button(Box<Layout<'a>>, &'a mut FrameStorage, Option<Rgb>),
    /// An embedded layout
    Visitor(Box<dyn Visitor + 'a>),
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
    pub fn single(widget: &'a mut dyn WidgetConfig) -> Self {
        let layout = LayoutType::Single(widget);
        Layout { layout }
    }

    /// Construct a single-item layout with alignment hints
    pub fn align_single(widget: &'a mut dyn WidgetConfig, hints: AlignHints) -> Self {
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
    /// This frame has dimensions according to [`SizeHandle::frame`].
    pub fn frame(data: &'a mut FrameStorage, child: Self) -> Self {
        let layout = LayoutType::Frame(Box::new(child), data);
        Layout { layout }
    }

    /// Construct a navigation frame around a sub-layout
    ///
    /// This frame has dimensions according to [`SizeHandle::frame`].
    pub fn nav_frame(data: &'a mut FrameStorage, child: Self) -> Self {
        let layout = LayoutType::NavFrame(Box::new(child), data);
        Layout { layout }
    }

    /// Construct a button frame around a sub-layout
    pub fn button(data: &'a mut FrameStorage, child: Self, color: Option<Rgb>) -> Self {
        let layout = LayoutType::Button(Box::new(child), data, color);
        Layout { layout }
    }

    /// Place a text element in the layout
    pub fn text(data: &'a mut TextStorage, text: &'a mut dyn TextApi, class: TextClass) -> Self {
        let layout = LayoutType::Visitor(Box::new(Text { data, text, class }));
        Layout { layout }
    }

    /// Construct a row/column layout over an iterator of layouts
    pub fn list<I, D, S>(list: I, direction: D, data: &'a mut S) -> Self
    where
        I: ExactSizeIterator<Item = Layout<'a>> + 'a,
        D: Directional,
        S: RowStorage,
    {
        let layout = LayoutType::Visitor(Box::new(List {
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
        W: WidgetConfig,
        D: Directional,
    {
        let layout = LayoutType::Visitor(Box::new(Slice {
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
        let layout = LayoutType::Visitor(Box::new(Grid {
            data,
            dim,
            children: iter,
        }));
        Layout { layout }
    }

    /// Get size rules for the given axis
    #[inline]
    pub fn size_rules(mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.size_rules_(sh, axis)
    }
    fn size_rules_(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        match &mut self.layout {
            LayoutType::None => SizeRules::EMPTY,
            LayoutType::Single(child) => child.size_rules(sh, axis),
            LayoutType::AlignSingle(child, _) => child.size_rules(sh, axis),
            LayoutType::AlignLayout(layout, _) => layout.size_rules_(sh, axis),
            LayoutType::Frame(child, storage) => {
                let frame_rules = sh.frame(axis.is_vertical());
                let child_rules = child.size_rules_(sh, axis);
                let (rules, offset, size) = frame_rules.surround_as_margin(child_rules);
                storage.offset.set_component(axis, offset);
                storage.size.set_component(axis, size);
                rules
            }
            LayoutType::NavFrame(child, storage) => {
                let frame_rules = sh.nav_frame(axis.is_vertical());
                let child_rules = child.size_rules_(sh, axis);
                let (rules, offset, size) = frame_rules.surround_as_margin(child_rules);
                storage.offset.set_component(axis, offset);
                storage.size.set_component(axis, size);
                rules
            }
            LayoutType::Button(child, storage, _) => {
                let frame_rules = sh.button_surround(axis.is_vertical());
                let child_rules = child.size_rules_(sh, axis);
                let (rules, offset, size) = frame_rules.surround_as_margin(child_rules);
                storage.offset.set_component(axis, offset);
                storage.size.set_component(axis, size);
                rules
            }
            LayoutType::Visitor(visitor) => visitor.size_rules(sh, axis),
        }
    }

    /// Apply a given `rect` to self
    #[inline]
    pub fn set_rect(mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.set_rect_(mgr, rect, align);
    }
    fn set_rect_(&mut self, mgr: &mut Manager, mut rect: Rect, align: AlignHints) {
        match &mut self.layout {
            LayoutType::None => (),
            LayoutType::Single(child) => child.set_rect(mgr, rect, align),
            LayoutType::AlignSingle(child, hints) => {
                let align = hints.combine(align);
                child.set_rect(mgr, rect, align);
            }
            LayoutType::AlignLayout(layout, hints) => {
                let align = hints.combine(align);
                layout.set_rect_(mgr, rect, align);
            }
            LayoutType::Frame(child, storage)
            | LayoutType::NavFrame(child, storage)
            | LayoutType::Button(child, storage, _) => {
                storage.rect = rect;
                rect.pos += storage.offset;
                rect.size -= storage.size;
                child.set_rect_(mgr, rect, align);
            }
            LayoutType::Visitor(layout) => layout.set_rect(mgr, rect, align),
        }
    }

    /// Return true if layout is up/left
    ///
    /// This is a lazy method of implementing tab order for reversible layouts.
    #[inline]
    pub fn is_reversed(mut self) -> bool {
        self.is_reversed_()
    }
    fn is_reversed_(&mut self) -> bool {
        match &mut self.layout {
            LayoutType::None => false,
            LayoutType::Single(_) | LayoutType::AlignSingle(_, _) => false,
            LayoutType::AlignLayout(layout, _)
            | LayoutType::Frame(layout, _)
            | LayoutType::NavFrame(layout, _)
            | LayoutType::Button(layout, _, _) => layout.is_reversed_(),
            LayoutType::Visitor(layout) => layout.is_reversed(),
        }
    }

    /// Draw a widget's children
    #[inline]
    pub fn draw(mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, state: InputState) {
        self.draw_(draw, mgr, state);
    }
    fn draw_(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, state: InputState) {
        let disabled = state.contains(InputState::DISABLED);
        match &mut self.layout {
            LayoutType::None => (),
            LayoutType::Single(child) => child.draw(draw, mgr, disabled),
            LayoutType::AlignSingle(child, _) => child.draw(draw, mgr, disabled),
            LayoutType::AlignLayout(layout, _) => layout.draw_(draw, mgr, state),
            LayoutType::Frame(child, storage) => {
                draw.outer_frame(storage.rect);
                child.draw_(draw, mgr, state);
            }
            LayoutType::NavFrame(child, storage) => {
                draw.nav_frame(storage.rect, state);
                child.draw_(draw, mgr, state);
            }
            LayoutType::Button(child, storage, color) => {
                draw.button(storage.rect, *color, state);
                child.draw_(draw, mgr, state);
            }
            LayoutType::Visitor(layout) => layout.draw(draw, mgr, state),
        }
    }
}

/// Implement row/column layout for children
struct List<'a, S, D, I> {
    data: &'a mut S,
    direction: D,
    children: I,
}

impl<'a, S: RowStorage, D: Directional, I> Visitor for List<'a, S, D, I>
where
    I: ExactSizeIterator<Item = Layout<'a>>,
{
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in (&mut self.children).enumerate() {
            solver.for_child(self.data, n, |axis| child.size_rules(sh, axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, self.data);

        for (n, child) in (&mut self.children).enumerate() {
            child.set_rect(mgr, setter.child_rect(self.data, n), align);
        }
    }

    fn is_reversed(&mut self) -> bool {
        self.direction.is_reversed()
    }

    fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, state: InputState) {
        for child in &mut self.children {
            child.draw(draw, mgr, state);
        }
    }
}

/// A row/column over a slice
struct Slice<'a, W: WidgetConfig, D: Directional> {
    data: &'a mut DynRowStorage,
    direction: D,
    children: &'a mut [W],
}

impl<'a, W: WidgetConfig, D: Directional> Visitor for Slice<'a, W, D> {
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in self.children.iter_mut().enumerate() {
            solver.for_child(self.data, n, |axis| child.size_rules(sh, axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, self.data);

        for (n, child) in self.children.iter_mut().enumerate() {
            child.set_rect(mgr, setter.child_rect(self.data, n), align);
        }
    }

    fn is_reversed(&mut self) -> bool {
        self.direction.is_reversed()
    }

    fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, state: InputState) {
        let solver = RowPositionSolver::new(self.direction);
        solver.for_children(self.children, draw.get_clip_rect(), |w| {
            w.draw(draw, mgr, state.contains(InputState::DISABLED))
        });
    }
}

/// Implement grid layout for children
struct Grid<'a, S, I> {
    data: &'a mut S,
    dim: GridDimensions,
    children: I,
}

impl<'a, S: GridStorage, I> Visitor for Grid<'a, S, I>
where
    I: Iterator<Item = (GridChildInfo, Layout<'a>)>,
{
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut solver = GridSolver::<Vec<_>, Vec<_>, _>::new(axis, self.dim, self.data);
        for (info, child) in &mut self.children {
            solver.for_child(self.data, info, |axis| child.size_rules(sh, axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let mut setter = GridSetter::<Vec<_>, Vec<_>, _>::new(rect, self.dim, align, self.data);
        for (info, child) in &mut self.children {
            child.set_rect(mgr, setter.child_rect(self.data, info), align);
        }
    }

    fn is_reversed(&mut self) -> bool {
        // TODO: replace is_reversed with direct implementation of spatial_nav
        false
    }

    fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, state: InputState) {
        for (_, child) in &mut self.children {
            child.draw(draw, mgr, state);
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

/// Layout storage for text element
#[derive(Clone, Default, Debug)]
pub struct TextStorage {
    /// Position of text
    pub pos: Coord,
}

struct Text<'a> {
    data: &'a mut TextStorage,
    text: &'a mut dyn TextApi,
    class: TextClass,
}

impl<'a> Visitor for Text<'a> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        size_handle.text_bound(self.text, self.class, axis)
    }

    fn set_rect(&mut self, _mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let halign = match self.class {
            TextClass::Button => Align::Center,
            _ => Align::Default,
        };
        self.data.pos = rect.pos;
        self.text.update_env(|env| {
            env.set_bounds(rect.size.into());
            env.set_align(align.unwrap_or(halign, Align::Center));
        });
    }

    fn is_reversed(&mut self) -> bool {
        false
    }

    fn draw(&mut self, draw: &mut dyn DrawHandle, _mgr: &ManagerState, state: InputState) {
        draw.text_effects(self.data.pos, self.text, self.class, state);
    }
}
