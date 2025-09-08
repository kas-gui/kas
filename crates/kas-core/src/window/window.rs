// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use super::{Decorations, Icon, Popup, PopupDescriptor, ResizeDirection, WindowId};
use crate::cast::Cast;
use crate::dir::{Direction, Directional};
use crate::event::{Command, ConfigCx, Event, EventCx, IsUsed, Scroll, Unused, Used};
use crate::geom::{Coord, Offset, Rect, Size};
use crate::layout::{self, Align, AlignHints, AxisInfo, SizeRules};
use crate::theme::{DrawCx, FrameStyle, SizeCx};
use crate::widgets::{Border, Label, TitleBar};
use crate::{Action, Events, Id, Layout, Role, RoleCx, Tile, TileExt, Widget};
use kas_macros::{impl_self, widget_set_rect};
use smallvec::SmallVec;

pub(crate) trait WindowErased {
    fn as_tile(&self) -> &dyn Tile;
    fn show_tooltip(&mut self, cx: &mut EventCx, id: Id, text: String);
    fn close_tooltip(&mut self, cx: &mut EventCx);
}

#[impl_self]
mod Window {
    /// The window widget
    ///
    /// This widget is the root of any UI tree used as a window. It manages
    /// window decorations.
    ///
    /// # Messages
    ///
    /// [`kas::messages::SetWindowTitle`] may be used to set the title.
    ///
    /// [`kas::messages::SetWindowIcon`] may be used to set the icon.
    #[widget]
    pub struct Window<Data: 'static> {
        core: widget_core!(),
        icon: Option<Icon>, // initial icon, if any
        decorations: Decorations,
        restrictions: (bool, bool),
        drag_anywhere: bool,
        transparent: bool,
        escapable: bool,
        alt_bypass: bool,
        disable_nav_focus: bool,
        #[widget]
        inner: Box<dyn Widget<Data = Data>>,
        #[widget(&())]
        tooltip: Popup<Label<String>>,
        #[widget(&())]
        title_bar: TitleBar,
        #[widget(&())]
        b_w: Border,
        #[widget(&())]
        b_e: Border,
        #[widget(&())]
        b_n: Border,
        #[widget(&())]
        b_s: Border,
        #[widget(&())]
        b_nw: Border,
        #[widget(&())]
        b_ne: Border,
        #[widget(&())]
        b_sw: Border,
        #[widget(&())]
        b_se: Border,
        bar_h: i32,
        dec_offset: Offset,
        dec_size: Size,
        popups: SmallVec<[(WindowId, PopupDescriptor, Offset); 16]>,
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

            // These methods don't return anything useful, but we are required to call them:
            let _ = self.b_w.size_rules(sizer.re(), axis);
            let _ = self.b_e.size_rules(sizer.re(), axis);
            let _ = self.b_n.size_rules(sizer.re(), axis);
            let _ = self.b_s.size_rules(sizer.re(), axis);
            let _ = self.b_nw.size_rules(sizer.re(), axis);
            let _ = self.b_ne.size_rules(sizer.re(), axis);
            let _ = self.b_se.size_rules(sizer.re(), axis);
            let _ = self.b_sw.size_rules(sizer.re(), axis);

            if matches!(self.decorations, Decorations::Border | Decorations::Toolkit) {
                let frame = sizer.frame(FrameStyle::Window, axis);
                let (rules, offset, size) = frame.surround(inner);
                self.dec_offset.set_component(axis, offset);
                self.dec_size.set_component(axis, size);
                rules
            } else {
                inner
            }
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            // Calculate position and size for nw, ne, and inner portions:
            let s_nw: Size = self.dec_offset.cast();
            let s_se = self.dec_size - s_nw;
            let mut s_in = rect.size - self.dec_size;
            let p_nw = rect.pos;
            let mut p_in = p_nw + self.dec_offset;
            let p_se = p_in + s_in;

            self.b_w.set_rect(
                cx,
                Rect::new(Coord(p_nw.0, p_in.1), Size(s_nw.0, s_in.1)),
                hints,
            );
            self.b_e.set_rect(
                cx,
                Rect::new(Coord(p_se.0, p_in.1), Size(s_se.0, s_in.1)),
                hints,
            );
            self.b_n.set_rect(
                cx,
                Rect::new(Coord(p_in.0, p_nw.1), Size(s_in.0, s_nw.1)),
                hints,
            );
            self.b_s.set_rect(
                cx,
                Rect::new(Coord(p_in.0, p_se.1), Size(s_in.0, s_se.1)),
                hints,
            );
            self.b_nw.set_rect(cx, Rect::new(p_nw, s_nw), hints);
            self.b_ne.set_rect(
                cx,
                Rect::new(Coord(p_se.0, p_nw.1), Size(s_se.0, s_nw.1)),
                hints,
            );
            self.b_se.set_rect(cx, Rect::new(p_se, s_se), hints);
            self.b_sw.set_rect(
                cx,
                Rect::new(Coord(p_nw.0, p_se.1), Size(s_nw.0, s_se.1)),
                hints,
            );

            if self.bar_h > 0 {
                let bar_size = Size(s_in.0, self.bar_h);
                self.title_bar
                    .set_rect(cx, Rect::new(p_in, bar_size), hints);
                p_in.1 += self.bar_h;
                s_in -= Size(0, self.bar_h);
            }
            self.inner.set_rect(cx, Rect::new(p_in, s_in), hints);
        }

        fn draw(&self, mut draw: DrawCx) {
            // Draw access keys first to prioritise their access key bindings
            for (_, popup, translation) in &self.popups {
                if let Some(child) = self.find_tile(&popup.id) {
                    // We use a new pass to control draw order and clip content:
                    let clip_rect = child.rect() - *translation;
                    draw.with_overlay(clip_rect, *translation, |draw| {
                        child.draw(draw);
                    });
                }
            }

            if self.dec_size != Size::ZERO {
                draw.frame(self.rect(), FrameStyle::Window, Default::default());
                if self.bar_h > 0 {
                    self.title_bar.draw(draw.re());
                }
            }
            self.inner.draw(draw.re());
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Window
        }

        fn probe(&self, coord: Coord) -> Id {
            for (_, popup, translation) in self.popups.iter().rev() {
                if let Some(widget) = self.inner.find_tile(&popup.id)
                    && let Some(id) = widget.try_probe(coord + *translation)
                {
                    return id;
                }
            }
            if self.bar_h > 0
                && let Some(id) = self.title_bar.try_probe(coord)
            {
                return id;
            }
            self.inner
                .try_probe(coord)
                .or_else(|| self.b_w.try_probe(coord))
                .or_else(|| self.b_e.try_probe(coord))
                .or_else(|| self.b_n.try_probe(coord))
                .or_else(|| self.b_s.try_probe(coord))
                .or_else(|| self.b_nw.try_probe(coord))
                .or_else(|| self.b_ne.try_probe(coord))
                .or_else(|| self.b_sw.try_probe(coord))
                .or_else(|| self.b_se.try_probe(coord))
                .unwrap_or_else(|| self.id())
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

            if self.alt_bypass {
                cx.config.alt_bypass = true;
            }

            if self.disable_nav_focus {
                cx.config.nav_focus = false;
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(Command::Escape, _) => {
                    if let Some(id) = self.popups.last().map(|desc| desc.0) {
                        cx.close_window(id);
                    } else if self.escapable {
                        cx.window_action(Action::CLOSE);
                    }
                    Used
                }
                Event::PressStart(_) if self.drag_anywhere => {
                    cx.drag_window();
                    Used
                }
                Event::Timer(handle) if handle == crate::event::Mouse::TIMER_HOVER => {
                    cx.hover_timer_expiry(self);
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(kas::messages::SetWindowTitle(title)) = cx.try_pop() {
                self.title_bar.set_title(cx, title);
                if self.decorations == Decorations::Server
                    && let Some(w) = cx.winit_window()
                {
                    w.set_title(self.title());
                }
            } else if let Some(kas::messages::SetWindowIcon(icon)) = cx.try_pop() {
                if self.decorations == Decorations::Server
                    && let Some(w) = cx.winit_window()
                {
                    w.set_window_icon(icon);
                    return; // do not set self.icon
                }
                self.icon = icon;
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, data: &Data, _: Scroll) {
            // Something was scrolled; update pop-up translations
            self.resize_popups(&mut cx.config_cx(), data);
        }
    }

    impl WindowErased for Self {
        fn as_tile(&self) -> &dyn Tile {
            self
        }

        fn show_tooltip(&mut self, cx: &mut EventCx, id: Id, text: String) {
            self.tooltip.inner.set_string(cx, text);
            self.tooltip.open(cx, &(), id, false);
        }

        fn close_tooltip(&mut self, cx: &mut EventCx) {
            self.tooltip.close(cx);
        }
    }

    impl std::fmt::Debug for Self {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Window")
                .field("core", &self.core)
                .field("title", &self.title_bar.title())
                .finish()
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
            escapable: false,
            alt_bypass: false,
            disable_nav_focus: false,
            inner: ui,
            tooltip: Popup::new(Label::default(), Direction::Down).align(Align::Center),
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

    /// Get the window's icon, if any
    pub(crate) fn icon(&mut self) -> Option<Icon> {
        self.icon.clone()
    }

    /// Set the window's icon (inline)
    ///
    /// Default: `None`
    pub fn with_icon(mut self, icon: impl Into<Option<Icon>>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Get the preference for window decorations
    pub fn decorations(&self) -> Decorations {
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

    /// Enable closure via <kbd>Escape</kbd> key
    pub fn escapable(mut self) -> Self {
        self.escapable = true;
        self
    }

    /// Enable <kbd>Alt</kbd> bypass
    ///
    /// Access keys usually require that <kbd>Alt</kbd> be held. This method
    /// allows access keys to be activated without holding <kbd>Alt</kbd>.
    pub fn with_alt_bypass(mut self) -> Self {
        self.alt_bypass = true;
        self
    }

    /// Disable navigation focus
    ///
    /// Usually, widgets may be focussed and this focus may be navigated using
    /// the <kbd>Tab</kbd> key. This method prevents widgets from gaining focus.
    pub fn without_nav_focus(mut self) -> Self {
        self.disable_nav_focus = true;
        self
    }

    /// Add a pop-up as a layer in the current window
    ///
    /// Each [`crate::Popup`] is assigned a [`WindowId`]; both are passed.
    pub(crate) fn add_popup(
        &mut self,
        cx: &mut ConfigCx,
        data: &Data,
        id: WindowId,
        popup: PopupDescriptor,
    ) {
        let index = 'index: {
            for i in 0..self.popups.len() {
                if self.popups[i].0 == id {
                    debug_assert_eq!(self.popups[i].1.id, popup.id);
                    self.popups[i].1 = popup;
                    break 'index i;
                }
            }

            let len = self.popups.len();
            self.popups.push((id, popup, Offset::ZERO));
            len
        };

        self.resize_popup(cx, data, index);
        cx.confirm_popup_is_sized(id);
        cx.action(self.id(), Action::REGION_MOVED);
    }

    /// Trigger closure of a pop-up
    ///
    /// If the given `id` refers to a pop-up, it should be closed.
    pub(crate) fn remove_popup(&mut self, cx: &mut ConfigCx, id: WindowId) {
        for i in 0..self.popups.len() {
            if id == self.popups[i].0 {
                self.popups.remove(i);
                cx.action(self.id(), Action::REGION_MOVED);
                return;
            }
        }
    }

    /// Resize popups
    ///
    /// This is called immediately after [`Layout::set_rect`] to resize
    /// existing pop-ups.
    pub(crate) fn resize_popups(&mut self, cx: &mut ConfigCx, data: &Data) {
        for i in 0..self.popups.len() {
            self.resize_popup(cx, data, i);
        }
    }

    /// Iterate over popups
    #[cfg(feature = "accesskit")]
    pub(crate) fn iter_popups(&self) -> impl Iterator<Item = &PopupDescriptor> {
        self.popups.iter().map(|(_, popup, _)| popup)
    }
}

impl<Data: 'static> Window<Data> {
    fn resize_popup(&mut self, cx: &mut ConfigCx, data: &Data, index: usize) {
        // Notation: p=point/coord, s=size, m=margin
        // r=window/root rect, c=anchor rect
        let r = self.rect();
        let popup = self.popups[index].1.clone();

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
        let place_out = |rp, rs, cp: i32, cs, ideal: i32, align| -> (i32, i32) {
            let mut size = ideal.max(cs).min(rs);
            let pos = match align {
                Align::Default | Align::TL => cp,
                Align::BR => cp + cs,
                Align::Center => cp + (cs - size) / 2,
                Align::Stretch => {
                    size = size.max(cs);
                    cp
                }
            };
            let pos = pos.min(rp + rs - size).max(rp);
            (pos, size)
        };

        let Some((c, t)) = self.as_tile().find_tile_rect(&popup.parent) else {
            return;
        };
        self.popups[index].2 = t;
        let r = r + t; // work in translated coordinate space
        let result = self.as_node(data).find_node(&popup.id, |mut node| {
            let mut cache = layout::SolveCache::find_constraints(node.re(), cx.size_cx());
            let ideal = cache.ideal(false);
            let m = cache.margins();

            let rect = if popup.direction.is_horizontal() {
                let (x, w) = place_in(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0, m.horiz);
                let (y, h) = place_out(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1, popup.align);
                Rect::new(Coord(x, y), Size::new(w, h))
            } else {
                let (x, w) = place_out(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0, popup.align);
                let (y, h) = place_in(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1, m.vert);
                Rect::new(Coord(x, y), Size::new(w, h))
            };

            cache.apply_rect(node.re(), cx, rect, false);
            cache.print_widget_heirarchy(node.as_tile());
        });

        // Event handlers expect that the popup's rect is now assigned.
        // If we were to try recovering we should remove the popup.
        assert!(result.is_some());
    }
}
