// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use crate::dir::Directional;
use crate::event::{ConfigMgr, EventMgr, Scroll};
use crate::geom::{Coord, Offset, Rect, Size};
use crate::layout::{self, AxisInfo, SizeRules};
use crate::theme::{DrawMgr, FrameStyle, SizeMgr};
use crate::title_bar::TitleBar;
use crate::{Action, Events, Icon, Layout, Node, Widget, WidgetExt, WidgetId};
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

/// Available decoration modes
///
/// See [`Window::decorations`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Decorations {
    /// No decorations
    ///
    /// The root widget is drawn as a simple rectangle with no borders.
    None,
    /// Add a simple themed border to the widget
    ///
    /// Probably looks better if [`Window::transparent`] is true.
    Border,
    /// Toolkit-drawn decorations
    ///
    /// Decorations will match the toolkit theme, not the platform theme.
    /// These decorations may not have all the same capabilities.
    ///
    /// Probably looks better if [`Window::transparent`] is true.
    Toolkit,
    /// Server-side decorations
    ///
    /// Decorations are drawn by the window manager, if available.
    Server,
}

impl_scope! {
    /// A support layer around a window
    ///
    /// TODO: there is currently no mechanism for adjusting window properties at
    /// run-time. The intention is to support sending a message like:
    /// `mgr.push(WindowCommand::SetTitle("New Title"));`. The problem is that
    /// this window representation is disconnected from winit::Window and has no
    /// mechanism for updating that. This may be easier to implement later.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[widget]
    pub struct Window<Data: 'static> {
        core: widget_core!(),
        icon: Option<Icon>,
        decorations: Decorations,
        restrictions: (bool, bool),
        drag_anywhere: bool,
        transparent: bool,
        #[widget(&())]
        title_bar: TitleBar,
        #[widget]
        w: Box<dyn Widget<Data = Data>>,
        bar_h: i32,
        dec_offset: Offset,
        dec_size: Size,
        popups: SmallVec<[(WindowId, kas::Popup, Offset); 16]>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let mut inner = self.w.size_rules(size_mgr.re(), axis);

            self.bar_h = 0;
            if matches!(self.decorations, Decorations::Toolkit) {
                let bar = self.title_bar.size_rules(size_mgr.re(), axis);
                if axis.is_horizontal() {
                    inner.max_with(bar);
                } else {
                    inner.append(bar);
                    self.bar_h = bar.min_size();
                }
            }
            if matches!(self.decorations, Decorations::Border | Decorations::Toolkit) {
                let frame = size_mgr.frame(FrameStyle::Window, axis);
                let (rules, offset, size) = frame.surround(inner);
                self.dec_offset.set_component(axis, offset);
                self.dec_size.set_component(axis, size);
                rules
            } else {
                inner
            }
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, mut rect: Rect) {
            self.core.rect = rect;
            rect.pos += self.dec_offset;
            rect.size -= self.dec_size;
            if self.bar_h > 0 {
                let bar_size = Size(rect.size.0, self.bar_h);
                self.title_bar.set_rect(mgr, Rect::new(rect.pos, bar_size));
                rect.pos.1 += self.bar_h;
                rect.size -= Size(0, self.bar_h);
            }
            self.w.set_rect(mgr, rect);
        }

        fn find_id(&mut self, _: Coord) -> Option<WidgetId> {
            unimplemented!()
        }

        fn draw(&mut self, _: DrawMgr) {
            unimplemented!()
        }
    }

    impl Self {
        pub(crate) fn find_id(&mut self, data: &Data, coord: Coord) -> Option<WidgetId> {
            if !self.core.rect.contains(coord) {
                return None;
            }
            for (_, popup, translation) in self.popups.iter_mut().rev() {
                if let Some(id) = self
                    .w
                    .find_node_mut(data, &popup.id)
                    .and_then(|mut w| w.find_id(coord + *translation))
                {
                    return Some(id);
                }
            }
            self.title_bar.find_id(coord)
                .or_else(|| self.w.find_id(coord))
                .or_else(|| Some(self.id()))
        }

        #[cfg(feature = "winit")]
        pub(crate) fn draw(&mut self, data: &Data, mut draw: DrawMgr) {
            if self.dec_size != Size::ZERO {
                draw.frame(self.core.rect, FrameStyle::Window, Default::default());
                if self.bar_h > 0 {
                    draw.recurse(&mut self.title_bar);
                }
            }
            draw.recurse(&mut self.w);
            for (_, popup, translation) in &self.popups {
                if let Some(mut widget) = self.w.find_node_mut(data, &popup.id) {
                    let clip_rect = widget.rect() - *translation;
                    draw.with_overlay(clip_rect, *translation, |draw| {
                        widget._draw(draw);
                    });
                }
            }
        }
    }

    impl Events for Self {
        type Data = Data;

        fn configure(&mut self, _: &Data, mgr: &mut ConfigMgr) {
            if mgr.platform().is_wayland() && self.decorations == Decorations::Server {
                // Wayland's base protocol does not support server-side decorations
                // TODO: Wayland has extensions for this; server-side is still
                // usually preferred where supported (e.g. KDE).
                self.decorations = Decorations::Toolkit;
            }
        }

        fn handle_scroll(&mut self, data: &Data, mgr: &mut EventMgr, _: Scroll) {
            // Something was scrolled; update pop-up translations
            mgr.config_mgr(|mgr| self.resize_popups(data, mgr));
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
            title_bar: TitleBar::new(title),
            w: ui,
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

    /// Get the window's icon, if any
    pub fn icon(&self) -> Option<&Icon> {
        self.icon.as_ref()
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
    pub fn add_popup(&mut self, data: &Data, mgr: &mut EventMgr, id: WindowId, popup: kas::Popup) {
        let index = self.popups.len();
        self.popups.push((id, popup, Offset::ZERO));
        mgr.config_mgr(|mgr| self.resize_popup(data, mgr, index));
        mgr.send_action(Action::REDRAW);
    }

    /// Trigger closure of a pop-up
    ///
    /// If the given `id` refers to a pop-up, it should be closed.
    pub fn remove_popup(&mut self, mgr: &mut EventMgr, id: WindowId) {
        for i in 0..self.popups.len() {
            if id == self.popups[i].0 {
                self.popups.remove(i);
                mgr.send_action(Action::REGION_MOVED);
                return;
            }
        }
    }

    /// Resize popups
    ///
    /// This is called immediately after [`Layout::set_rect`] to resize
    /// existing pop-ups.
    pub fn resize_popups(&mut self, data: &Data, mgr: &mut ConfigMgr) {
        for i in 0..self.popups.len() {
            self.resize_popup(data, mgr, i);
        }
    }
}

// Search for a widget by `id`. On success, return that widget's [`Rect`] and
// the translation of its children.
fn find_rect(widget: Node<'_>, id: WidgetId, mut translation: Offset) -> Option<(Rect, Offset)> {
    if let Some(i) = widget.find_child_index(&id) {
        if let Some(w) = widget.re().get_child(i) {
            translation += widget.translation();
            return find_rect(w, id, translation);
        }
    }

    if widget.eq_id(&id) {
        if widget.translation() != Offset::ZERO {
            // Unvalidated: does this cause issues with the parent's event handlers?
            log::warn!(
                "Parent of pop-up {} has non-zero translation",
                widget.identify()
            );
        }

        let rect = widget.rect();
        Some((rect, translation))
    } else {
        None
    }
}

impl<Data: 'static> Window<Data> {
    fn resize_popup(&mut self, data: &Data, mgr: &mut ConfigMgr, index: usize) {
        // Notation: p=point/coord, s=size, m=margin
        // r=window/root rect, c=anchor rect
        let r = self.core.rect;
        let (_, ref mut popup, ref mut translation) = self.popups[index];

        let (c, t) = find_rect(self.w.as_node(data), popup.parent.clone(), Offset::ZERO).unwrap();
        *translation = t;
        let r = r + t; // work in translated coordinate space
        let mut widget = self.w.find_node_mut(data, &popup.id).unwrap();
        let mut cache = layout::SolveCache::find_constraints(widget.re(), mgr.size_mgr());
        let ideal = cache.ideal(false);
        let m = cache.margins();

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
        let rect = if popup.direction.is_horizontal() {
            let (x, w) = place_in(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0, m.horiz);
            let (y, h) = place_out(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1);
            Rect::new(Coord(x, y), Size::new(w, h))
        } else {
            let (x, w) = place_out(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0);
            let (y, h) = place_in(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1, m.vert);
            Rect::new(Coord(x, y), Size::new(w, h))
        };

        cache.apply_rect(widget.re(), mgr, rect, false);
        cache.print_widget_heirarchy(widget.as_node());
    }
}
