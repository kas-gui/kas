// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use super::{FrameStyle, MarkStyle, SelectionStyle, SizeMgr, TextClass, ThemeSize};
use crate::dir::Direction;
use crate::draw::color::Rgb;
use crate::draw::{Draw, DrawIface, DrawShared, DrawSharedImpl, ImageId, PassType};
use crate::event::{ConfigCx, EventState};
use crate::geom::{Offset, Rect};
use crate::text::{TextApi, TextDisplay};
use crate::{autoimpl, Action, Layout, WidgetId};
use std::convert::AsRef;
use std::ops::{Bound, Range, RangeBounds};
use std::time::Instant;

/// Optional background colour
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum Background {
    /// Use theme/feature's default
    #[default]
    Default,
    /// Error state
    Error,
    /// A given color
    Rgb(Rgb),
}

/// Draw interface
///
/// This interface is provided to widgets in [`crate::Layout::draw`].
/// Lower-level interfaces may be accessed through [`Self::draw_device`].
///
/// `DrawCx` is not a `Copy` or `Clone` type; instead it may be "reborrowed"
/// via [`Self::re_id`] or [`Self::re_clone`].
///
/// -   `draw.check_box(&*self, self.state);` — note `&*self` to convert from to
///     `&W` from `&mut W`, since the latter would cause borrow conflicts
pub struct DrawCx<'a> {
    h: &'a mut dyn ThemeDraw,
    id: WidgetId,
}

impl<'a> DrawCx<'a> {
    /// Reborrow with a new lifetime and new `id`
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    #[inline(always)]
    pub fn re_id<'b>(&'b mut self, id: WidgetId) -> DrawCx<'b>
    where
        'a: 'b,
    {
        DrawCx { h: self.h, id }
    }

    /// Reborrow with a new lifetime and same `id`
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    #[inline(always)]
    pub fn re_clone<'b>(&'b mut self) -> DrawCx<'b>
    where
        'a: 'b,
    {
        DrawCx {
            h: self.h,
            id: self.id.clone(),
        }
    }

    /// Recurse drawing to a child
    #[inline]
    pub fn recurse(&mut self, child: &mut (impl Layout + ?Sized)) {
        child.draw(self.re_id(child.id_ref().clone()));
    }

    /// Construct from a [`DrawCx`] and [`EventState`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(h: &'a mut dyn ThemeDraw, id: WidgetId) -> Self {
        DrawCx { h, id }
    }

    /// Access event-management state
    pub fn ev_state(&mut self) -> &mut EventState {
        self.h.components().2
    }

    /// Access a [`SizeMgr`]
    pub fn size_mgr(&mut self) -> SizeMgr {
        SizeMgr::new(self.h.components().0)
    }

    /// Access a [`ConfigCx`]
    pub fn config_cx<F: FnOnce(&mut ConfigCx) -> T, T>(&mut self, f: F) -> T {
        let (sh, draw, ev) = self.h.components();
        let mut cx = ConfigCx::new(sh, draw.shared(), ev);
        f(&mut cx)
    }

    /// Access a [`DrawShared`]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.h.components().1.shared()
    }

    /// Access the low-level draw device
    ///
    /// Note: this drawing API is modular, with limited functionality in the
    /// base trait [`Draw`]. To access further functionality, it is necessary
    /// to downcast with [`crate::draw::DrawIface::downcast_from`].
    pub fn draw_device(&mut self) -> &mut dyn Draw {
        self.h.components().1
    }

    /// Access the low-level draw device (implementation type)
    ///
    /// The implementing type must be specified. See [`DrawIface::downcast_from`].
    pub fn draw_iface<DS: DrawSharedImpl>(&mut self) -> Option<DrawIface<DS>> {
        DrawIface::downcast_from(self.draw_device())
    }

    /// Draw to a new pass
    ///
    /// Adds a new draw pass for purposes of enforcing draw order. Content of
    /// the new pass will be drawn after content in the parent pass.
    pub fn with_pass<F: FnOnce(DrawCx)>(&mut self, f: F) {
        let clip_rect = self.h.get_clip_rect();
        let id = self.id.clone();
        self.h.new_pass(
            clip_rect,
            Offset::ZERO,
            PassType::Clip,
            Box::new(|h| f(DrawCx { h, id })),
        );
    }

    /// Draw to a new pass with clipping and offset (e.g. for scrolling)
    ///
    /// Adds a new draw pass of type [`PassType::Clip`], with draw operations
    /// clipped to `rect` and translated by `offset.
    pub fn with_clip_region<F: FnOnce(DrawCx)>(&mut self, rect: Rect, offset: Offset, f: F) {
        let id = self.id.clone();
        self.h.new_pass(
            rect,
            offset,
            PassType::Clip,
            Box::new(|h| f(DrawCx { h, id })),
        );
    }

    /// Draw to a new pass as an overlay (e.g. for pop-up menus)
    ///
    /// Adds a new draw pass of type [`PassType::Overlay`], with draw operations
    /// clipped to `rect`.
    ///
    /// The theme is permitted to enlarge the `rect` for the purpose of drawing
    /// a frame or shadow around this overlay, thus the
    /// [`Self::get_clip_rect`] may be larger than expected.
    pub fn with_overlay<F: FnOnce(DrawCx)>(&mut self, rect: Rect, offset: Offset, f: F) {
        let id = self.id.clone();
        self.h.new_pass(
            rect,
            offset,
            PassType::Overlay,
            Box::new(|h| f(DrawCx { h, id })),
        );
    }

    /// Target area for drawing
    ///
    /// Drawing is restricted to this [`Rect`], which may be the whole window, a
    /// [clip region](Self::with_clip_region) or an
    /// [overlay](Self::with_overlay). This may be used to cull hidden
    /// items from lists inside a scrollable view.
    pub fn get_clip_rect(&mut self) -> Rect {
        self.h.get_clip_rect()
    }

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions are given by [`SizeMgr::frame`].
    pub fn frame(&mut self, rect: Rect, style: FrameStyle, bg: Background) {
        self.h.frame(&self.id, rect, style, bg)
    }

    /// Draw a separator in the given `rect`
    pub fn separator(&mut self, rect: Rect) {
        self.h.separator(rect);
    }

    /// Draw a selection highlight / frame
    ///
    /// Adjusts the background color and/or draws a line around the given rect.
    /// In the latter case, a margin of size [`SizeMgr::inner_margins`] around
    /// `rect` is expected.
    pub fn selection(&mut self, rect: Rect, style: SelectionStyle) {
        self.h.selection(rect, style);
    }

    /// Draw text
    ///
    /// Text is drawn from `rect.pos` and clipped to `rect`. If the text
    /// scrolls, `rect` should be the size of the whole text, not the window.
    ///
    /// [`ConfigCx::text_set_size`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text(&mut self, rect: Rect, text: impl AsRef<TextDisplay>, class: TextClass) {
        self.h.text(&self.id, rect, text.as_ref(), class);
    }

    /// Draw text with effects
    ///
    /// Text is drawn from `rect.pos` and clipped to `rect`. If the text
    /// scrolls, `rect` should be the size of the whole text, not the window.
    ///
    /// [`Self::text`] already supports *font* effects: bold,
    /// emphasis, text size. In addition, this method supports underline and
    /// strikethrough effects.
    ///
    /// [`ConfigCx::text_set_size`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_effects(&mut self, rect: Rect, text: &dyn TextApi, class: TextClass) {
        self.h.text_effects(&self.id, rect, text, class);
    }

    /// Draw some text using the standard font, with a subset selected
    ///
    /// Other than visually highlighting the selection, this method behaves
    /// identically to [`Self::text`]. It is likely to be replaced in the
    /// future by a higher-level API.
    pub fn text_selected<R: RangeBounds<usize>>(
        &mut self,
        rect: Rect,
        text: impl AsRef<TextDisplay>,
        range: R,
        class: TextClass,
    ) {
        let start = match range.start_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(n) => *n + 1,
            Bound::Excluded(n) => *n,
            Bound::Unbounded => usize::MAX,
        };
        let range = Range { start, end };
        self.h
            .text_selected_range(&self.id, rect, text.as_ref(), range, class);
    }

    /// Draw an edit marker at the given `byte` index on this `text`
    ///
    /// The text cursor is draw from `rect.pos` and clipped to `rect`. If the text
    /// scrolls, `rect` should be the size of the whole text, not the window.
    ///
    /// [`ConfigCx::text_set_size`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_cursor(
        &mut self,
        rect: Rect,
        text: impl AsRef<TextDisplay>,
        class: TextClass,
        byte: usize,
    ) {
        self.h
            .text_cursor(&self.id, rect, text.as_ref(), class, byte);
    }

    /// Draw UI element: check box (without label)
    ///
    /// The check box is a small visual element, typically a distinctive square
    /// box with or without a "check" selection mark.
    ///
    /// The theme may animate transitions. To achieve this, `last_change` should be
    /// the time of the last state change caused by the user, or none when the
    /// last state change was programmatic.
    pub fn check_box(&mut self, rect: Rect, checked: bool, last_change: Option<Instant>) {
        self.h.check_box(&self.id, rect, checked, last_change);
    }

    /// Draw UI element: radio box (without label)
    ///
    /// The radio box is a small visual element, typically a disinctive
    /// circular box with or without a "radio" selection mark.
    ///
    /// The theme may animate transitions. To achieve this, `last_change` should be
    /// the time of the last state change caused by the user, or none when the
    /// last state change was programmatic.
    pub fn radio_box(&mut self, rect: Rect, checked: bool, last_change: Option<Instant>) {
        self.h.radio_box(&self.id, rect, checked, last_change);
    }

    /// Draw UI element: mark
    pub fn mark(&mut self, rect: Rect, style: MarkStyle) {
        self.h.mark(&self.id, rect, style);
    }

    /// Draw UI element: scroll bar
    pub fn scroll_bar<W: Layout>(&mut self, track_rect: Rect, handle: &W, dir: Direction) {
        self.h
            .scroll_bar(&self.id, handle.id_ref(), track_rect, handle.rect(), dir);
    }

    /// Draw UI element: slider
    pub fn slider<W: Layout>(&mut self, track_rect: Rect, handle: &W, dir: Direction) {
        self.h
            .slider(&self.id, handle.id_ref(), track_rect, handle.rect(), dir);
    }

    /// Draw UI element: progress bar
    ///
    /// -   `rect`: area of whole widget
    /// -   `dir`: direction of progress bar
    /// -   `state`: highlighting information
    /// -   `value`: progress value, between 0.0 and 1.0
    pub fn progress_bar(&mut self, rect: Rect, dir: Direction, value: f32) {
        self.h.progress_bar(&self.id, rect, dir, value);
    }

    /// Draw an image
    pub fn image(&mut self, rect: Rect, id: ImageId) {
        self.h.image(id, rect);
    }
}

impl<'a> std::ops::BitOrAssign<Action> for DrawCx<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: Action) {
        self.h.components().2.send_action(action);
    }
}

/// Theme drawing implementation
///
/// # Theme extension
///
/// Most themes will not want to implement *everything*, but rather derive
/// not-explicitly-implemented methods from a base theme. This may be achieved
/// with the [`kas::extends`](crate::extends) macro:
/// ```ignore
/// #[extends(ThemeDraw, base = self.base())]
/// impl ThemeDraw {
///     // only implement some methods here
/// }
/// ```
/// Note: [`Self::components`] must be implemented
/// explicitly since this method returns references.
///
/// If Rust had stable specialization + GATs + negative trait bounds we could
/// allow theme extension without macros as follows.
/// <details>
///
/// ```ignore
/// #![feature(generic_associated_types)]
/// #![feature(specialization)]
/// # use kas_core::geom::Rect;
/// # use kas_core::theme::ThemeDraw;
/// /// Provides a default implementation of each theme method over a base theme
/// pub trait ThemeDrawExtends: ThemeDraw {
///     /// Type of base implementation
///     type Base<'a>: ThemeDraw where Self: 'a;
///
///     /// Access the base theme
///     fn base<'a>(&'a mut self) -> Self::Base<'a>;
/// }
///
/// // Note: we may need negative trait bounds here to avoid conflict with impl for Box<H>
/// impl<D: ThemeDrawExtends> ThemeDraw for D {
///     default fn get_clip_rect(&mut self) -> Rect {
///         self.base().get_clip_rect()
///     }
///
///     // And so on for other methods...
/// }
/// ```
/// </details>
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[autoimpl(for<H: trait + ?Sized> Box<H>)]
pub trait ThemeDraw {
    /// Access components: [`ThemeSize`], [`Draw`], [`EventState`]
    fn components(&mut self) -> (&dyn ThemeSize, &mut dyn Draw, &mut EventState);

    /// Construct a new pass
    fn new_pass<'a>(
        &mut self,
        rect: Rect,
        offset: Offset,
        class: PassType,
        f: Box<dyn FnOnce(&mut dyn ThemeDraw) + 'a>,
    );

    /// Target area for drawing
    ///
    /// Drawing is restricted to this [`Rect`]. Affected by [`Self::new_pass`].
    /// This may be used to cull hidden items from lists inside a scrollable view.
    fn get_clip_rect(&mut self) -> Rect;

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions are given by [`ThemeSize::frame`].
    fn frame(&mut self, id: &WidgetId, rect: Rect, style: FrameStyle, bg: Background);

    /// Draw a separator in the given `rect`
    fn separator(&mut self, rect: Rect);

    /// Draw a selection highlight / frame
    fn selection(&mut self, rect: Rect, style: SelectionStyle);

    /// Draw text
    ///
    /// [`ConfigCx::text_set_size`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text(&mut self, id: &WidgetId, rect: Rect, text: &TextDisplay, class: TextClass);

    /// Draw text with effects
    ///
    /// [`ThemeDraw::text`] already supports *font* effects: bold,
    /// emphasis, text size. In addition, this method supports underline and
    /// strikethrough effects.
    ///
    /// [`ConfigCx::text_set_size`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text_effects(&mut self, id: &WidgetId, rect: Rect, text: &dyn TextApi, class: TextClass);

    /// Method used to implement [`DrawCx::text_selected`]
    fn text_selected_range(
        &mut self,
        id: &WidgetId,
        rect: Rect,
        text: &TextDisplay,
        range: Range<usize>,
        class: TextClass,
    );

    /// Draw an edit marker at the given `byte` index on this `text`
    ///
    /// [`ConfigCx::text_set_size`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text_cursor(
        &mut self,
        id: &WidgetId,
        rect: Rect,
        text: &TextDisplay,
        class: TextClass,
        byte: usize,
    );

    /// Draw UI element: check box
    ///
    /// The check box is a small visual element, typically a distinctive square
    /// box with or without a "check" selection mark.
    ///
    /// The theme may animate transitions. To achieve this, `last_change` should be
    /// the time of the last state change caused by the user, or none when the
    /// last state change was programmatic.
    fn check_box(&mut self, id: &WidgetId, rect: Rect, checked: bool, last_change: Option<Instant>);

    /// Draw UI element: radio button
    ///
    /// The radio box is a small visual element, typically a disinctive
    /// circular box with or without a "radio" selection mark.
    ///
    /// The theme may animate transitions. To achieve this, `last_change` should be
    /// the time of the last state change caused by the user, or none when the
    /// last state change was programmatic.
    fn radio_box(&mut self, id: &WidgetId, rect: Rect, checked: bool, last_change: Option<Instant>);

    /// Draw UI element: mark
    fn mark(&mut self, id: &WidgetId, rect: Rect, style: MarkStyle);

    /// Draw UI element: scroll bar
    ///
    /// -   `id`: [`WidgetId`] of the bar
    /// -   `id2`: [`WidgetId`] of the handle
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of bar
    fn scroll_bar(
        &mut self,
        id: &WidgetId,
        id2: &WidgetId,
        rect: Rect,
        h_rect: Rect,
        dir: Direction,
    );

    /// Draw UI element: slider
    ///
    /// -   `id`: [`WidgetId`] of the bar
    /// -   `id2`: [`WidgetId`] of the handle
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of slider (currently only LTR or TTB)
    fn slider(&mut self, id: &WidgetId, id2: &WidgetId, rect: Rect, h_rect: Rect, dir: Direction);

    /// Draw UI element: progress bar
    ///
    /// -   `id`: [`WidgetId`] of the bar
    /// -   `rect`: area of whole widget
    /// -   `dir`: direction of progress bar
    /// -   `value`: progress value, between 0.0 and 1.0
    fn progress_bar(&mut self, id: &WidgetId, rect: Rect, dir: Direction, value: f32);

    /// Draw an image
    fn image(&mut self, id: ImageId, rect: Rect);
}

#[cfg(test)]
mod test {
    use super::*;

    fn _draw_ext(mut draw: DrawCx) {
        // We can't call this method without constructing an actual ThemeDraw.
        // But we don't need to: we just want to test that methods are callable.

        let _scale = draw.size_mgr().scale_factor();

        let text = crate::text::Text::new("sample");
        let class = TextClass::Label(false);
        draw.text_selected(Rect::ZERO, &text, .., class)
    }
}
