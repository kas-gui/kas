// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use crate::cast::Cast;
use crate::decorations::{Border, Decorations, TitleBar};
use crate::dir::Directional;
use crate::event::{ConfigCx, Event, EventCx, ResizeDirection, Response, Scroll};
use crate::geom::{Coord, Offset, Rect, Size};
use crate::layout::{self, AxisInfo, SizeRules};
use crate::theme::{DrawCx, FrameStyle, SizeCx};
use crate::{Action, Events, Icon, Layout, LayoutExt, Widget, WidgetId};
use kas_macros::impl_scope;
use smallvec::SmallVec;
use std::num::NonZeroU32;

/// Identifier for a window or pop-up
///
/// Identifiers should always be unique.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WindowId(NonZeroU32);

impl WindowId {
    /// Construct a [`WindowId`]
    ///
    /// Only for use by the shell!
    #[allow(unused)]
    pub(crate) fn new(n: NonZeroU32) -> WindowId {
        WindowId(n)
    }
}

/// Commands supported by the [`Window`]
///
/// This may be sent as a message from any widget in the window.
#[derive(Clone, Debug)]
pub enum WindowCommand {
    /// Change the window's title
    SetTitle(String),
    /// Change the window's icon
    SetIcon(Option<Icon>),
}

impl_scope! {
    /// A support layer around a window
    ///
    /// To change window properties at run-time, send a [`WindowCommand`] from a
    /// child widget.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[widget]
    pub struct Window<Data: 'static> {
        core: widget_core!(),
        icon: Option<Icon>, // initial icon, if any
        decorations: Decorations,
        restrictions: (bool, bool),
        drag_anywhere: bool,
        transparent: bool,
        #[widget]
        inner: Box<dyn Widget<Data = Data>>,
        #[widget(&())]
        title_bar: TitleBar,
        #[widget(&())] b_w: Border,
        #[widget(&())] b_e: Border,
        #[widget(&())] b_n: Border,
        #[widget(&())] b_s: Border,
        #[widget(&())] b_nw: Border,
        #[widget(&())] b_ne: Border,
        #[widget(&())] b_sw: Border,
        #[widget(&())] b_se: Border,
        bar_h: i32,
        dec_offset: Offset,
        dec_size: Size,
        popups: SmallVec<[(WindowId, kas::Popup, Offset); 16]>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut inner = self.inner.size_rules(sizer.re(), axis);

            self.bar_h = 0;
            if matches!(self.decorations, Decorations::Toolkit) {
                let bar = self.title_bar.size_rules(sizer.re(), axis);
                if axis.is_horizontal() {
                    inner.max_with(bar);
                } else {
                    inner.append(bar);
                    self.bar_h = bar.min_size();
                }
            }
            if matches!(self.decorations, Decorations::Border | Decorations::Toolkit) {
                // We would call size_rules on Border widgets here if it did anything
                let frame = sizer.frame(FrameStyle::Window, axis);
                let (rules, offset, size) = frame.surround(inner);
                self.dec_offset.set_component(axis, offset);
                self.dec_size.set_component(axis, size);
                rules
            } else {
                inner
            }
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            // Calculate position and size for nw, ne, and inner portions:
            let s_nw: Size = self.dec_offset.cast();
            let s_se = self.dec_size - s_nw;
            let mut s_in = rect.size - self.dec_size;
            let p_nw = rect.pos;
            let mut p_in = p_nw + self.dec_offset;
            let p_se = p_in + s_in;

            self.b_w.set_rect(cx, Rect::new(Coord(p_nw.0, p_in.1), Size(s_nw.0, s_in.1)));
            self.b_e.set_rect(cx, Rect::new(Coord(p_se.0, p_in.1), Size(s_se.0, s_in.1)));
            self.b_n.set_rect(cx, Rect::new(Coord(p_in.0, p_nw.1), Size(s_in.0, s_nw.1)));
            self.b_s.set_rect(cx, Rect::new(Coord(p_in.0, p_se.1), Size(s_in.0, s_se.1)));
            self.b_nw.set_rect(cx, Rect::new(p_nw, s_nw));
            self.b_ne.set_rect(cx, Rect::new(Coord(p_se.0, p_nw.1), Size(s_se.0, s_nw.1)));
            self.b_se.set_rect(cx, Rect::new(p_se, s_se));
            self.b_sw.set_rect(cx, Rect::new(Coord(p_nw.0, p_se.1), Size(s_nw.0, s_se.1)));

            if self.bar_h > 0 {
                let bar_size = Size(s_in.0, self.bar_h);
                self.title_bar.set_rect(cx, Rect::new(p_in, bar_size));
                p_in.1 += self.bar_h;
                s_in -= Size(0, self.bar_h);
            }
            self.inner.set_rect(cx, Rect::new(p_in, s_in));
        }

        fn find_id(&mut self, _: Coord) -> Option<WidgetId> {
            unimplemented!()
        }

        fn draw(&mut self, _: DrawCx) {
            unimplemented!()
        }
    }

    impl Self {
        pub(crate) fn find_id(&mut self, data: &Data, coord: Coord) -> Option<WidgetId> {
            if !self.core.rect.contains(coord) {
                return None;
            }
            for (_, popup, translation) in self.popups.iter_mut().rev() {
                if let Some(Some(id)) = self.inner.as_node(data).for_id(&popup.id, |mut node| node.find_id(coord + *translation)) {
                    return Some(id);
                }
            }
            self.inner.find_id(coord)
                .or_else(|| self.title_bar.find_id(coord))
                .or_else(|| self.b_w.find_id(coord))
                .or_else(|| self.b_e.find_id(coord))
                .or_else(|| self.b_n.find_id(coord))
                .or_else(|| self.b_s.find_id(coord))
                .or_else(|| self.b_nw.find_id(coord))
                .or_else(|| self.b_ne.find_id(coord))
                .or_else(|| self.b_sw.find_id(coord))
                .or_else(|| self.b_se.find_id(coord))
                .or_else(|| Some(self.id()))
        }

        #[cfg(winit)]
        pub(crate) fn draw(&mut self, data: &Data, mut draw: DrawCx) {
            if self.dec_size != Size::ZERO {
                draw.frame(self.core.rect, FrameStyle::Window, Default::default());
                if self.bar_h > 0 {
                    draw.recurse(&mut self.title_bar);
                }
            }
            draw.recurse(&mut self.inner);
            for (_, popup, translation) in &self.popups {
                self.inner.as_node(data).for_id(&popup.id, |mut node| {
                    let clip_rect = node.rect() - *translation;
                    draw.with_overlay(clip_rect, *translation, |draw| {
                        node._draw(draw);
                    });
                });
            }
        }
    }

    impl Events for Self {
        type Data = Data;

        fn configure(&mut self, cx: &mut ConfigCx) {
            if cx.platform().is_wayland() && self.decorations == Decorations::Server {
                // Wayland's base protocol does not support server-side decorations
                // TODO: Wayland has extensions for this; server-side is still
                // usually preferred where supported (e.g. KDE).
                self.decorations = Decorations::Toolkit;
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> Response {
            match event {
                Event::PressStart { .. } if self.drag_anywhere => {
                    cx.drag_window();
                    Response::Used
                }
                _ => Response::Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(cmd) = cx.try_pop() {
                match cmd {
                    WindowCommand::SetTitle(title) => {
                        *cx |= self.title_bar.set_title(title);
                        #[cfg(winit)]
                        if self.decorations == Decorations::Server {
                            if let Some(w) = cx.winit_window() {
                                w.set_title(self.title());
                            }
                        }
                    }
                    WindowCommand::SetIcon(icon) => {
                        #[cfg(winit)]
                        if self.decorations == Decorations::Server {
                            if let Some(w) = cx.winit_window() {
                                w.set_window_icon(icon);
                                return; // do not set self.icon
                            }
                        }
                        self.icon = icon;
                    }
                }
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, data: &Data, _: Scroll) {
            // Something was scrolled; update pop-up translations
            cx.config_cx(|cx| self.resize_popups(cx, data));
        }
    }
}

impl<Data: 'static> Window<Data> {
    /// Construct a window with a `W: Widget` and a title
    pub fn new(ui: impl Widget<Data = Data> + 'static, title: impl ToString) -> Self {
        Self::new_boxed(Box::new(ui), title)
    }

    /// Construct a window from a boxed `ui` widget and a `title`
    pub fn new_boxed(ui: Box<dyn Widget<Data = Data>>, title: impl ToString) -> Self {
        Window {
            core: Default::default(),
            icon: None,
            decorations: Decorations::Server,
            restrictions: (true, false),
            drag_anywhere: true,
            transparent: false,
            inner: ui,
            title_bar: TitleBar::new(title),
            b_w: Border::new(ResizeDirection::West),
            b_e: Border::new(ResizeDirection::East),
            b_n: Border::new(ResizeDirection::North),
            b_s: Border::new(ResizeDirection::South),
            b_nw: Border::new(ResizeDirection::NorthWest),
            b_ne: Border::new(ResizeDirection::NorthEast),
            b_sw: Border::new(ResizeDirection::SouthWest),
            b_se: Border::new(ResizeDirection::SouthEast),
            bar_h: 0,
            dec_offset: Default::default(),
            dec_size: Default::default(),
            popups: Default::default(),
        }
    }

    /// Get the window's title
    pub fn title(&self) -> &str {
        self.title_bar.title()
    }

    /// Take the window's icon, if any
    pub(crate) fn take_icon(&mut self) -> Option<Icon> {
        self.icon.take()
    }

    /// Set the window's icon (inline)
    ///
    /// Default: `None`
    pub fn with_icon(mut self, icon: impl Into<Option<Icon>>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Get the preference for window decorations
    pub fn decorations(&self) -> crate::Decorations {
        self.decorations
    }

    /// Set the preference for window decorations
    ///
    /// "Windowing" platforms (i.e. not mobile or web) usually include a
    /// title-bar, icons and potentially side borders. These are known as
    /// **decorations**.
    ///
    /// This controls the *preferred* type of decorations. The resulting
    /// behaviour is platform-dependent.
    ///
    /// Default: [`Decorations::Server`].
    pub fn with_decorations(mut self, decorations: Decorations) -> Self {
        self.decorations = decorations;
        self
    }

    /// Get window resizing restrictions: `(restrict_min, restrict_max)`
    pub fn restrictions(&self) -> (bool, bool) {
        self.restrictions
    }

    /// Whether to limit the maximum size of a window
    ///
    /// All widgets' size rules allow calculation of two sizes: the minimum
    /// size and the ideal size. Windows are initially sized to the ideal size.
    ///
    /// If `restrict_min`, the window may not be sized below the minimum size.
    /// Default value: `true`.
    ///
    /// If `restrict_max`, the window may not be sized above the ideal size.
    /// Default value: `false`.
    pub fn with_restrictions(mut self, restrict_min: bool, restrict_max: bool) -> Self {
        self.restrictions = (restrict_min, restrict_max);
        let resizable = !restrict_min || !restrict_max;
        self.b_w.set_resizable(resizable);
        self.b_e.set_resizable(resizable);
        self.b_n.set_resizable(resizable);
        self.b_s.set_resizable(resizable);
        self.b_nw.set_resizable(resizable);
        self.b_ne.set_resizable(resizable);
        self.b_se.set_resizable(resizable);
        self.b_sw.set_resizable(resizable);
        self
    }

    /// Get "drag anywhere" state
    pub fn drag_anywhere(&self) -> bool {
        self.drag_anywhere
    }

    /// Whether to allow dragging the window from the background
    ///
    /// If true, then any unhandled click+drag in the window may be used to
    /// drag the window on supported platforms. Default value: `true`.
    pub fn with_drag_anywhere(mut self, drag_anywhere: bool) -> Self {
        self.drag_anywhere = drag_anywhere;
        self
    }

    /// Get whether this window should use transparent rendering
    pub fn transparent(&self) -> bool {
        self.transparent
    }

    /// Whether the window supports transparency
    ///
    /// If true, painting with `alpha < 1.0` makes the background visible.
    /// Additionally, window draw targets are cleared to transparent. This does
    /// not stop theme elements from drawing a solid background.
    ///
    /// Note: results may vary by platform. Current output does *not* use
    /// pre-multiplied alpha which *some* platforms expect, thus pixels with
    /// partial transparency may have incorrect appearance.
    ///
    /// Default: `false`.
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Add a pop-up as a layer in the current window
    ///
    /// Each [`crate::Popup`] is assigned a [`WindowId`]; both are passed.
    pub fn add_popup(&mut self, cx: &mut EventCx, data: &Data, id: WindowId, popup: kas::Popup) {
        let index = self.popups.len();
        self.popups.push((id, popup, Offset::ZERO));
        cx.config_cx(|cx| self.resize_popup(cx, data, index));
        cx.send_action(Action::REDRAW);
    }

    /// Trigger closure of a pop-up
    ///
    /// If the given `id` refers to a pop-up, it should be closed.
    pub fn remove_popup(&mut self, cx: &mut EventCx, id: WindowId) {
        for i in 0..self.popups.len() {
            if id == self.popups[i].0 {
                self.popups.remove(i);
                cx.send_action(Action::REGION_MOVED);
                return;
            }
        }
    }

    /// Resize popups
    ///
    /// This is called immediately after [`Layout::set_rect`] to resize
    /// existing pop-ups.
    pub fn resize_popups(&mut self, cx: &mut ConfigCx, data: &Data) {
        for i in 0..self.popups.len() {
            self.resize_popup(cx, data, i);
        }
    }
}

// Search for a widget by `id`. On success, return that widget's [`Rect`] and
// the translation of its children.
fn find_rect(widget: &dyn Layout, id: WidgetId, mut translation: Offset) -> Option<(Rect, Offset)> {
    let mut widget = widget;
    loop {
        if widget.eq_id(&id) {
            if widget.translation() != Offset::ZERO {
                // Unvalidated: does this cause issues with the parent's event handlers?
                log::warn!(
                    "Parent of pop-up {} has non-zero translation",
                    widget.identify()
                );
            }

            let rect = widget.rect();
            return Some((rect, translation));
        } else if let Some(child) = widget
            .find_child_index(&id)
            .and_then(|i| widget.get_child(i))
        {
            translation += widget.translation();
            widget = child;
            continue;
        } else {
            return None;
        }
    }
}

impl<Data: 'static> Window<Data> {
    fn resize_popup(&mut self, cx: &mut ConfigCx, data: &Data, index: usize) {
        // Notation: p=point/coord, s=size, m=margin
        // r=window/root rect, c=anchor rect
        let r = self.core.rect;
        let (_, ref mut popup, ref mut translation) = self.popups[index];

        let is_reversed = popup.direction.is_reversed();
        let place_in = |rp, rs: i32, cp: i32, cs: i32, ideal, m: (u16, u16)| -> (i32, i32) {
            let m: (i32, i32) = (m.0.into(), m.1.into());
            let before: i32 = cp - (rp + m.1);
            let before = before.max(0);
            let after = (rs - (cs + before + m.0)).max(0);
            if after >= ideal {
                if is_reversed && before >= ideal {
                    (cp - ideal - m.1, ideal)
                } else {
                    (cp + cs + m.0, ideal)
                }
            } else if before >= ideal {
                (cp - ideal - m.1, ideal)
            } else if before > after {
                (rp, before)
            } else {
                (cp + cs + m.0, after)
            }
        };
        let place_out = |rp, rs, cp: i32, cs, ideal: i32| -> (i32, i32) {
            let pos = cp.min(rp + rs - ideal).max(rp);
            let size = ideal.max(cs).min(rs);
            (pos, size)
        };

        let (c, t) = find_rect(self.inner.as_layout(), popup.parent.clone(), Offset::ZERO).unwrap();
        *translation = t;
        let r = r + t; // work in translated coordinate space
        self.inner.as_node(data).for_id(&popup.id, |mut node| {
            let mut cache = layout::SolveCache::find_constraints(node.re(), cx.size_cx());
            let ideal = cache.ideal(false);
            let m = cache.margins();

            let rect = if popup.direction.is_horizontal() {
                let (x, w) = place_in(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0, m.horiz);
                let (y, h) = place_out(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1);
                Rect::new(Coord(x, y), Size::new(w, h))
            } else {
                let (x, w) = place_out(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0);
                let (y, h) = place_in(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1, m.vert);
                Rect::new(Coord(x, y), Size::new(w, h))
            };

            cache.apply_rect(node.re(), cx, rect, false);
            cache.print_widget_heirarchy(node.as_layout());
        });
    }
}
