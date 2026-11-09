// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use super::icon::Icon;
use super::{Decorations, Popup, PopupDescriptor, ResizeDirection, WindowId};
use crate::cast::Cast;
use crate::dir::{Direction, Directional};
use crate::event::{Command, ConfigCx, Event, EventCx, IsUsed, Scroll, Unused, Used};
use crate::geom::{Coord, Offset, Rect, Size};
use crate::layout::{self, Align, AlignHints, AxisInfo, SizeRules};
use crate::runner::AppData;
use crate::theme::{DrawCx, FrameStyle, SizeCx};
use crate::widgets::adapt::MapAny;
use crate::widgets::{Border, Label, TitleBar};
use crate::{Events, Id, Layout, Role, RoleCx, Tile, TileExt, Widget, WidgetCoreRect};
use kas_macros::{autoimpl, impl_self};
use smallvec::SmallVec;

// TODO(Rust): replace with type-alias-impl-trait when available
#[cfg(feature = "accesskit")]
pub(crate) struct PopupIterator<'a>(usize, &'a [(WindowId, PopupDescriptor, Offset)]);
#[cfg(feature = "accesskit")]
impl<'a> Iterator for PopupIterator<'a> {
    type Item = &'a PopupDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.0;
        if i < self.1.len() {
            self.0 = i + 1;
            Some(&self.1[i].1)
        } else {
            None
        }
    }
}

#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub(crate) trait WindowErased: Tile {
    /// Get the cached window title
    fn title(&self) -> &str;
    fn properties(&mut self) -> &mut Properties;
    fn show_tooltip(&mut self, cx: &mut EventCx, id: Id, text: String);
    fn close_tooltip(&mut self, cx: &mut EventCx);

    /// Iterate over popups
    #[cfg(feature = "accesskit")]
    fn iter_popups(&self) -> PopupIterator<'_>;
}

#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub(crate) trait WindowWidget: WindowErased + Widget {
    /// Add or update a popup
    ///
    /// A popup is emulated as a layer in the current window. If there is an
    /// existing popup with the given `id` then it is updated with the given
    /// `popup` descriptor, otherwise a new popup is created.
    fn add_or_update_popup(
        &mut self,
        cx: &mut SizeCx,
        data: &Self::Data,
        id: WindowId,
        popup: PopupDescriptor,
    );

    /// Trigger closure of a pop-up
    ///
    /// If the given `id` refers to a pop-up, it should be closed.
    fn remove_popup(&mut self, cx: &mut SizeCx, id: WindowId);

    /// Resize popups
    ///
    /// This is called immediately after [`Layout::set_rect`] to resize
    /// existing pop-ups.
    fn resize_popups(&mut self, cx: &mut SizeCx, data: &Self::Data);
}

/// Window properties
///
/// It is recommended to construct this using one of the provided constructors,
/// for example [`Properties::top_defaults()`] or as follows:
/// ```
/// # use kas_core::window::Properties;
/// let props = Properties {
///     transparent: true,
///     ..Properties::modal_defaults()
/// };
/// ```
#[derive(Clone, Debug)]
pub struct Properties {
    /// The window's icon
    pub icon: Option<Icon>,
    /// The window's decoration preferences
    ///
    /// Desktop platforms (i.e. not mobile or web) usually include a
    /// title-bar, icons and potentially side borders. These are known as
    /// **decorations**.
    ///
    /// Defaults to `None`. In this case behaviour is platform dependent:
    ///
    /// -   [`Decorations::Toolkit`] on Wayland (TODO: support detection of
    ///     whether the compositor can provide decorations)
    /// -   [`Decorations::Server`] on other desktop platforms
    /// -   [`Decorations::None`] on mobile and web
    pub decorations: Option<Decorations>,
    /// If set, the window may not be resized smaller than its minimum size.
    pub restrict_min: bool,
    /// If set, the window may not be resized larger than its ideal size.
    pub restrict_max: bool,
    /// Whether to allow dragging the window from the background
    ///
    /// If true, then any unhandled click+drag in the window may be used to
    /// drag the window on supported platforms.
    pub drag_anywhere: bool,
    /// Whether the window supports transparency
    ///
    /// If true, painting with `alpha < 1.0` makes the background visible.
    /// Additionally, window draw targets are cleared to transparent. This does
    /// not stop theme elements from drawing a solid background.
    ///
    /// Note: results may vary by platform. Current output does *not* use
    /// pre-multiplied alpha which *some* platforms expect, thus pixels with
    /// partial transparency may have incorrect appearance.
    pub transparent: bool,
    /// Whether the window can be closed with the <kbd>Escape</kbd> key
    pub escapable: bool,
    /// Enable <kbd>Alt</kbd> bypass
    ///
    /// Access keys usually require that <kbd>Alt</kbd> be held. This method
    /// allows access keys to be activated without holding <kbd>Alt</kbd>.
    pub alt_bypass: bool,
    /// Disable navigation focus
    ///
    /// Usually, widgets may be focussed and this focus may be navigated using
    /// the <kbd>Tab</kbd> key. This method prevents widgets from gaining focus.
    pub disable_nav_focus: bool,
    /// Window is modal
    ///
    /// If set, this window is considered modal: it is owned by parent window,
    /// is not listed on the taskbar, and prevents interaction with the parent
    /// until this window has been closed.
    ///
    /// [`Self::parent`] must also be set for this to have any effect. The
    /// [`EventCx`] methods adding a child window do this automatically.
    ///
    /// **Implementation status:** planned (currently does nothing).
    pub modal: bool,
    /// The parent window, if any
    ///
    /// This is set automatically by [`EventCx`] methods which add a window.
    pub parent: Option<WindowId>,
}

impl Properties {
    /// Defaults intended for top-level windows
    ///
    /// Sets `restrict_min: true`; uses type-defaults for everything else.
    pub fn top_defaults() -> Self {
        Properties {
            icon: None,
            decorations: None,
            restrict_min: true,
            restrict_max: false,
            drag_anywhere: false,
            transparent: false,
            escapable: false,
            alt_bypass: false,
            disable_nav_focus: false,
            modal: false,
            parent: None,
        }
    }

    /// Defaults intended for top-level dialog windows
    ///
    /// Sets `drag_anywhere: true`, `escapable: true`; other
    /// settings are inherited from [`Self::top_defaults`].
    pub fn dialog_defaults() -> Self {
        Properties {
            drag_anywhere: true,
            escapable: true,
            ..Self::top_defaults()
        }
    }

    /// Defaults intended for modal windows
    ///
    /// Sets `modal: true`; other settings are inherited from
    /// [`Self::dialog_defaults`].
    pub fn modal_defaults() -> Self {
        Properties {
            drag_anywhere: true,
            escapable: true,
            modal: true,
            ..Self::top_defaults()
        }
    }

    /// Defaults intended for fixed-size modal windows
    ///
    /// Sets `restrict_max: true`; other settings are inherited from
    /// [`Self::modal_defaults`].
    pub fn fixed_modal_defaults() -> Self {
        Properties {
            restrict_max: true,
            ..Self::modal_defaults()
        }
    }

    /// Defaults intended for widget apps
    ///
    /// This restricts size (min and max), disables decorations, enables
    /// [`Self::drag_anywhere`] and [`Self::transparent`] and disables
    /// navigation focus.
    pub fn widget_defaults() -> Self {
        Properties {
            decorations: Some(Decorations::None),
            restrict_max: true,
            drag_anywhere: true,
            transparent: true,
            disable_nav_focus: true,
            ..Self::top_defaults()
        }
    }

    /// Get the parent if modal
    pub fn modal_parent(&self) -> Option<WindowId> {
        if self.modal && self.parent.is_some() {
            return self.parent;
        } else if cfg!(debug_assertions) && self.modal {
            log::warn!("Properties: model = true but parent = None");
        }
        None
    }
}

#[impl_self]
mod Window {
    /// The window widget
    ///
    /// This widget is the root of any UI tree used as a window. It manages
    /// window decorations.
    #[widget]
    pub struct Window<Data: AppData> {
        core: widget_core!(),
        props: Properties,
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
        bar_margin: i32,
        dec_offset: Offset,
        dec_size: Size,
        popups: SmallVec<[(WindowId, PopupDescriptor, Offset); 16]>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, mut axis: AxisInfo) -> SizeRules {
            let frame_width = self.dec_size.extract(axis.flipped());
            axis.map_other(|w| w - frame_width);
            let mut inner = self.inner.size_rules(cx, axis);

            self.bar_h = 0;
            if matches!(cx.decorations, Decorations::Toolkit) {
                let bar = self.title_bar.size_rules(cx, axis);
                if axis.is_horizontal() {
                    inner.max_with(bar);
                } else {
                    self.bar_h = bar.min_size();
                    self.bar_margin = bar.margins_i32().1.max(inner.margins_i32().0);
                    inner = bar.appended(inner);
                }
            }

            // These methods don't return anything useful, but we are required to call them:
            let _ = self.b_w.size_rules(cx, axis);
            let _ = self.b_e.size_rules(cx, axis);
            let _ = self.b_n.size_rules(cx, axis);
            let _ = self.b_s.size_rules(cx, axis);
            let _ = self.b_nw.size_rules(cx, axis);
            let _ = self.b_ne.size_rules(cx, axis);
            let _ = self.b_se.size_rules(cx, axis);
            let _ = self.b_sw.size_rules(cx, axis);

            if matches!(cx.decorations, Decorations::Border | Decorations::Toolkit) {
                let frame = cx.frame(FrameStyle::Window, axis);
                let (rules, offset, size) = frame.surround(inner);
                self.dec_offset.set_component(axis, offset);
                self.dec_size.set_component(axis, size);
                rules
            } else {
                inner
            }
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            self.core.set_rect(rect);
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
                p_in.1 += self.bar_h + self.bar_margin;
                s_in -= Size(0, self.bar_h + self.bar_margin);
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
    }

    impl Events for Self {
        type Data = Data;

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

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.decorations = if let Some(mode) = self.props.decorations {
                mode
            } else if cx.platform().is_wayland() {
                // Wayland's base protocol does not support server-side decorations
                // TODO: Wayland has extensions for this; server-side is still
                // usually preferred where supported (e.g. KDE).
                Decorations::Toolkit
            } else if cx.platform().is_desktop() {
                Decorations::Server
            } else {
                Decorations::None
            };

            if self.props.alt_bypass {
                cx.config.alt_bypass = true;
            }

            if self.props.disable_nav_focus {
                cx.config.nav_focus = false;
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(Command::Escape, _) => {
                    if let Some(id) = self.popups.last().map(|desc| desc.0) {
                        cx.close_window(id);
                    } else if self.props.escapable {
                        cx.top_window().close();
                    }
                    Used
                }
                Event::PressStart(press) if self.props.drag_anywhere => press.drag_window(cx),
                Event::Timer(handle) if handle == crate::event::Mouse::TIMER_TOOLTIP => {
                    cx.timer_expiry_tooltip(self);
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, data: &Data, _: Scroll) {
            // Something was scrolled; update pop-up translations
            self.resize_popups(&mut cx.size_cx(), data);
        }
    }

    impl WindowErased for Self {
        fn title(&self) -> &str {
            self.title_bar.title()
        }

        fn properties(&mut self) -> &mut Properties {
            &mut self.props
        }

        fn show_tooltip(&mut self, cx: &mut EventCx, id: Id, text: String) {
            self.tooltip.inner.set_text(cx, text);
            self.tooltip.open(cx, &(), id, false);
        }

        fn close_tooltip(&mut self, cx: &mut EventCx) {
            self.tooltip.close(cx);
        }

        #[cfg(feature = "accesskit")]
        fn iter_popups(&self) -> PopupIterator<'_> {
            PopupIterator(0, &self.popups)
        }
    }

    impl std::fmt::Debug for Self {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Window").field("core", &self.core).finish()
        }
    }
}

impl Window<()> {
    /// Map data type.
    ///
    /// Expectation: the window will be configured and sized after this.
    pub(crate) fn map_any<Data: AppData>(self) -> Window<Data> {
        Window {
            core: Default::default(),
            props: self.props,
            // TODO(opt): maybe we shouldn't box here?
            inner: Box::new(MapAny::new(self.inner)),
            tooltip: self.tooltip,
            title_bar: self.title_bar,
            b_w: self.b_w,
            b_e: self.b_e,
            b_n: self.b_n,
            b_s: self.b_s,
            b_nw: self.b_nw,
            b_ne: self.b_ne,
            b_sw: self.b_sw,
            b_se: self.b_se,
            bar_h: 0,
            bar_margin: 0,
            dec_offset: Default::default(),
            dec_size: Default::default(),
            popups: self.popups,
        }
    }
}

impl<Data: AppData> Window<Data> {
    /// Construct a window from a boxed `ui` widget and a `title`
    pub fn new(ui: Box<dyn Widget<Data = Data>>, title: String, props: Properties) -> Self {
        let resizable = !props.restrict_min || !props.restrict_max;

        let mut w = Window {
            core: Default::default(),
            props,
            inner: ui,
            tooltip: Popup::new(Label::new("".to_string()), Direction::Down).align(Align::Center),
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
            bar_margin: 0,
            dec_offset: Default::default(),
            dec_size: Default::default(),
            popups: Default::default(),
        };

        w.b_w.set_resizable(resizable);
        w.b_e.set_resizable(resizable);
        w.b_n.set_resizable(resizable);
        w.b_s.set_resizable(resizable);
        w.b_nw.set_resizable(resizable);
        w.b_ne.set_resizable(resizable);
        w.b_se.set_resizable(resizable);
        w.b_sw.set_resizable(resizable);

        w
    }
}

impl<Data: AppData> WindowWidget for Window<Data> {
    fn add_or_update_popup(
        &mut self,
        cx: &mut SizeCx,
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
        cx.region_moved();
    }

    fn remove_popup(&mut self, cx: &mut SizeCx, id: WindowId) {
        for i in 0..self.popups.len() {
            if id == self.popups[i].0 {
                self.popups.remove(i);
                cx.region_moved();
                return;
            }
        }
    }

    fn resize_popups(&mut self, cx: &mut SizeCx, data: &Data) {
        for i in 0..self.popups.len() {
            self.resize_popup(cx, data, i);
        }
    }
}

impl<Data: AppData> Window<Data> {
    fn resize_popup(&mut self, cx: &mut SizeCx, data: &Data, index: usize) {
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
        let result = Widget::as_node(self, data).find_node(&popup.id, |mut node| {
            let mut cache = layout::SolveCache::default();
            cache.find_constraints(node.re(), cx);
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
            cache.print_widget_hierarchy(node.as_tile());
        });

        // Event handlers expect that the popup's rect is now assigned.
        // If we were to try recovering we should remove the popup.
        assert!(result.is_some());
    }
}
