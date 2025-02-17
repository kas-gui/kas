// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout visitor

// Methods have to take `&mut self`
#![allow(clippy::wrong_self_convention)]

use super::{AlignHints, AxisInfo, SizeRules};
use super::{GridCellInfo, GridDimensions, GridSetter, GridSolver, GridStorage};
use super::{RowSetter, RowSolver, RowStorage};
use super::{RulesSetter, RulesSolver};
use crate::event::ConfigCx;
use crate::geom::{Coord, Offset, Rect, Size};
use crate::theme::{Background, DrawCx, FrameStyle, SizeCx};
use crate::Id;
use crate::{dir::Directional, Layout};

/// A list of [`Layout`]
///
/// This is templated over `cell_info: C` where `C = ()` for lists or
/// `C = GridCellInfo` for grids.
#[allow(clippy::len_without_is_empty)]
pub trait LayoutList<C> {
    /// List length
    fn len(&self) -> usize;

    /// Access an item
    fn get_item(&mut self, index: usize) -> Option<&mut dyn Layout> {
        self.get_info_item(index).map(|pair| pair.1)
    }

    fn get_info_item(&mut self, index: usize) -> Option<(C, &mut dyn Layout)>;
}

impl<C> LayoutList<C> for () {
    #[inline]
    fn len(&self) -> usize {
        0
    }

    #[inline]
    fn get_info_item(&mut self, _index: usize) -> Option<(C, &mut dyn Layout)> {
        None
    }
}

/// A layout visitor
///
/// Objects are generated by [`layout`] syntax. These all have limited lifetime.
///
/// [`layout`]: crate::widget#layout-1
pub struct Visitor<V: Layout>(V);

/// These methods would be free functions, but `Layout` is a useful namespace
impl<'a> Visitor<Box<dyn Layout + 'a>> {
    /// Construct a single-item layout
    pub fn single(widget: &'a mut dyn Layout) -> Visitor<impl Layout + 'a> {
        Visitor(Single { widget })
    }

    /// Construct a frame around a sub-layout
    ///
    /// This frame has dimensions according to [`SizeCx::frame`].
    pub fn frame<C: Layout + 'a>(
        storage: &'a mut FrameStorage,
        child: C,
        style: FrameStyle,
        bg: Background,
    ) -> Visitor<impl Layout + 'a> {
        Visitor(Frame {
            child,
            storage,
            style,
            bg,
        })
    }

    /// Construct a row/column layout over an iterator of layouts
    pub fn list<L, D, S>(list: L, direction: D, data: &'a mut S) -> Visitor<impl Layout + 'a>
    where
        L: LayoutList<()> + 'a,
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
    pub fn float<L: LayoutList<()> + 'a>(list: L) -> Visitor<impl Layout + 'a> {
        Visitor(Float { children: list })
    }

    /// Construct a grid layout over an iterator of `(cell, layout)` items
    pub fn grid<L, S>(
        children: L,
        dim: GridDimensions,
        data: &'a mut S,
    ) -> Visitor<impl Layout + 'a>
    where
        L: LayoutList<GridCellInfo> + 'a,
        S: GridStorage,
    {
        Visitor(Grid {
            data,
            dim,
            children,
        })
    }
}

impl<V: Layout> Visitor<V> {
    /// Apply alignment
    pub fn align(self, hints: AlignHints) -> Visitor<impl Layout> {
        Visitor(Align { child: self, hints })
    }

    /// Apply alignment and squash
    pub fn pack<'a>(
        self,
        hints: AlignHints,
        storage: &'a mut PackStorage,
    ) -> Visitor<impl Layout + 'a>
    where
        V: 'a,
    {
        Visitor(Pack {
            child: self,
            hints,
            storage,
        })
    }
}

impl<V: Layout> Visitor<V> {
    /// Get size rules for the given axis
    ///
    /// This method is identical to [`Layout::size_rules`].
    #[inline]
    pub fn size_rules(mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.0.size_rules(sizer, axis)
    }

    /// Apply a given `rect` to self
    ///
    /// The caller is expected to call `widget_set_rect!(rect);`.
    /// In other respects, this functions identically to [`Layout::set_rect`].
    #[inline]
    pub fn set_rect(mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        self.0.set_rect(cx, rect, hints);
    }

    /// Translate a coordinate to an [`Id`]
    ///
    /// The caller is expected to
    ///
    /// 1.  Return `None` if `!self.rect().contains(coord)`
    /// 2.  Translate `coord`: `let coord = coord + self.translation();`
    /// 3.  Call `try_probe` (this method), returning its result if not `None`
    /// 4.  Otherwise return `Some(self.id())`
    #[inline]
    pub fn try_probe(mut self, coord: Coord) -> Option<Id> {
        self.0.try_probe(coord)
    }

    /// Draw a widget and its children
    ///
    /// This method is identical to [`Layout::draw`].
    #[inline]
    pub fn draw(mut self, draw: DrawCx) {
        self.0.draw(draw);
    }
}

impl<V: Layout> Layout for Visitor<V> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.0.size_rules(sizer, axis)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        self.0.set_rect(cx, rect, hints);
    }

    fn try_probe(&mut self, coord: Coord) -> Option<Id> {
        self.0.try_probe(coord)
    }

    fn draw(&mut self, draw: DrawCx) {
        self.0.draw(draw);
    }
}

struct Single<'a> {
    widget: &'a mut dyn Layout,
}

impl<'a> Layout for Single<'a> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.widget.size_rules(sizer, axis)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        self.widget.set_rect(cx, rect, hints);
    }

    fn try_probe(&mut self, coord: Coord) -> Option<Id> {
        self.widget.try_probe(coord)
    }

    fn draw(&mut self, draw: DrawCx) {
        self.widget.draw(draw);
    }
}

struct Align<C: Layout> {
    child: C,
    hints: AlignHints,
}

impl<C: Layout> Layout for Align<C> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        self.child.size_rules(sizer, axis)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        let hints = self.hints.combine(hints);
        self.child.set_rect(cx, rect, hints);
    }

    fn try_probe(&mut self, coord: Coord) -> Option<Id> {
        self.child.try_probe(coord)
    }

    fn draw(&mut self, draw: DrawCx) {
        self.child.draw(draw);
    }
}

struct Pack<'a, C: Layout> {
    child: C,
    hints: AlignHints,
    storage: &'a mut PackStorage,
}

impl<'a, C: Layout> Layout for Pack<'a, C> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let rules = self.child.size_rules(sizer, axis);
        self.storage.size.set_component(axis, rules.ideal_size());
        rules
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        let rect = self
            .hints
            .combine(hints)
            .complete_default()
            .aligned_rect(self.storage.size, rect);
        self.child.set_rect(cx, rect, hints);
    }

    fn try_probe(&mut self, coord: Coord) -> Option<Id> {
        self.child.try_probe(coord)
    }

    fn draw(&mut self, draw: DrawCx) {
        self.child.draw(draw);
    }
}

struct Frame<'a, C: Layout> {
    child: C,
    storage: &'a mut FrameStorage,
    style: FrameStyle,
    bg: Background,
}

impl<'a, C: Layout> Layout for Frame<'a, C> {
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let child_rules = self
            .child
            .size_rules(sizer.re(), self.storage.child_axis(axis));
        self.storage
            .size_rules(sizer, axis, child_rules, self.style)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        self.storage.rect = rect;
        let child_rect = Rect {
            pos: rect.pos + self.storage.offset,
            size: rect.size - self.storage.size,
        };
        self.child.set_rect(cx, child_rect, hints);
    }

    fn try_probe(&mut self, coord: Coord) -> Option<Id> {
        self.child.try_probe(coord)
    }

    fn draw(&mut self, mut draw: DrawCx) {
        draw.frame(self.storage.rect, self.style, self.bg);
        self.child.draw(draw);
    }
}

/// Implement row/column layout for children
struct List<'a, L, D, S> {
    children: L,
    direction: D,
    data: &'a mut S,
}

impl<'a, L, D: Directional, S: RowStorage> Layout for List<'a, L, D, S>
where
    L: LayoutList<()> + 'a,
{
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for i in 0..self.children.len() {
            if let Some(child) = self.children.get_item(i) {
                solver.for_child(self.data, i, |axis| child.size_rules(sizer.re(), axis));
            }
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, self.data);

        for i in 0..self.children.len() {
            if let Some(child) = self.children.get_item(i) {
                child.set_rect(cx, setter.child_rect(self.data, i), hints);
            }
        }
    }

    fn try_probe(&mut self, coord: Coord) -> Option<Id> {
        // TODO(opt): more efficient search strategy?
        for i in 0..self.children.len() {
            if let Some(child) = self.children.get_item(i) {
                if let Some(id) = child.try_probe(coord) {
                    return Some(id);
                }
            }
        }
        None
    }

    fn draw(&mut self, mut draw: DrawCx) {
        for i in 0..self.children.len() {
            if let Some(child) = self.children.get_item(i) {
                child.draw(draw.re());
            }
        }
    }
}

/// Float layout
struct Float<L> {
    children: L,
}

impl<L> Layout for Float<L>
where
    L: LayoutList<()>,
{
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let mut rules = SizeRules::EMPTY;
        for i in 0..self.children.len() {
            if let Some(child) = self.children.get_item(i) {
                rules = rules.max(child.size_rules(sizer.re(), axis));
            }
        }
        rules
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        for i in 0..self.children.len() {
            if let Some(child) = self.children.get_item(i) {
                child.set_rect(cx, rect, hints);
            }
        }
    }

    fn try_probe(&mut self, coord: Coord) -> Option<Id> {
        for i in 0..self.children.len() {
            if let Some(child) = self.children.get_item(i) {
                if let Some(id) = child.try_probe(coord) {
                    return Some(id);
                }
            }
        }
        None
    }

    fn draw(&mut self, mut draw: DrawCx) {
        let mut iter = (0..self.children.len()).rev();
        if let Some(first) = iter.next() {
            if let Some(child) = self.children.get_item(first) {
                child.draw(draw.re());
            }
        }
        for i in iter {
            if let Some(child) = self.children.get_item(i) {
                draw.with_pass(|draw| child.draw(draw));
            }
        }
    }
}

/// Implement grid layout for children
struct Grid<'a, S, L> {
    data: &'a mut S,
    dim: GridDimensions,
    children: L,
}

impl<'a, S: GridStorage, L> Layout for Grid<'a, S, L>
where
    L: LayoutList<GridCellInfo> + 'a,
{
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let mut solver = GridSolver::<Vec<_>, Vec<_>, _>::new(axis, self.dim, self.data);
        for i in 0..self.children.len() {
            if let Some((info, child)) = self.children.get_info_item(i) {
                solver.for_child(self.data, info, |axis| child.size_rules(sizer.re(), axis));
            }
        }
        solver.finish(self.data)
    }

    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        let mut setter = GridSetter::<Vec<_>, Vec<_>, _>::new(rect, self.dim, self.data);
        for i in 0..self.children.len() {
            if let Some((info, child)) = self.children.get_info_item(i) {
                child.set_rect(cx, setter.child_rect(self.data, info), hints);
            }
        }
    }

    fn try_probe(&mut self, coord: Coord) -> Option<Id> {
        // TODO(opt): more efficient search strategy?
        for i in 0..self.children.len() {
            if let Some(child) = self.children.get_item(i) {
                if let Some(id) = child.try_probe(coord) {
                    return Some(id);
                }
            }
        }
        None
    }

    fn draw(&mut self, mut draw: DrawCx) {
        for i in (0..self.children.len()).rev() {
            if let Some(child) = self.children.get_item(i) {
                child.draw(draw.re());
            }
        }
    }
}

/// Layout storage for pack
#[derive(Clone, Default, Debug)]
pub struct PackStorage {
    /// Ideal size
    pub size: Size,
}

/// Layout storage for frame
#[derive(Clone, Default, Debug)]
pub struct FrameStorage {
    /// Size used by frame (sum of widths of borders)
    pub size: Size,
    /// Offset of frame contents from parent position
    pub offset: Offset,
    /// [`Rect`] assigned to whole frame
    ///
    /// NOTE: for a top-level layout component this is redundant with the
    /// widget's rect. For frames deeper within a widget's layout we *could*
    /// instead recalculate this (in every draw call etc.).
    pub rect: Rect,
}
impl FrameStorage {
    /// Calculate child's "other axis" size
    pub fn child_axis(&self, mut axis: AxisInfo) -> AxisInfo {
        axis.sub_other(self.size.extract(axis.flipped()));
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
