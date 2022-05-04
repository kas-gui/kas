// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use kas::layout;
use kas::prelude::*;
use kas::Icon;
use kas::{Future, WindowId};
use smallvec::SmallVec;

impl_scope! {
    /// The main instantiation of the [`Window`] trait.
    #[autoimpl(Clone ignore self.popups, self.drop where W: Clone)]
    #[autoimpl(Debug ignore self.drop, self.icon)]
    #[widget(layout = self.w;)]
    pub struct Window<W: Widget + 'static> {
        #[widget_core]
        core: CoreData,
        restrict_dimensions: (bool, bool),
        title: String,
        #[widget]
        w: W,
        popups: SmallVec<[(WindowId, kas::Popup); 16]>,
        drop: Option<(Box<dyn FnMut(&mut W)>, UpdateHandle)>,
        icon: Option<Icon>,
    }

    impl Layout for Self {
        #[inline]
        fn draw(&mut self, mut draw: DrawMgr) {
            draw.recurse(&mut self.w);
            for (_, popup) in &self.popups {
                if let Some(widget) = self.w.find_widget_mut(&popup.id) {
                    draw.with_overlay(widget.rect(), |mut draw| {
                        draw.recurse(widget);
                    });
                }
            }
        }
    }

    impl Widget for Self {
        #[inline]
        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            for popup in self.popups.iter_mut().rev() {
                if let Some(id) = self.w.find_widget_mut(&popup.1.id).and_then(|w| w.find_id(coord)) {
                    return Some(id);
                }
            }
            self.w.find_id(coord).or(Some(self.id()))
        }

    }

    impl<W: Widget + 'static> kas::Window for Window<W> {
        fn title(&self) -> &str {
            &self.title
        }

        fn icon(&self) -> Option<Icon> {
            self.icon.clone()
        }

        fn restrict_dimensions(&self) -> (bool, bool) {
            self.restrict_dimensions
        }

        fn add_popup(&mut self, mgr: &mut EventMgr, id: WindowId, popup: kas::Popup) {
            let index = self.popups.len();
            self.popups.push((id, popup));
            mgr.set_rect_mgr(|mgr| self.resize_popup(mgr, index));
            mgr.send_action(TkAction::REDRAW);
        }

        fn remove_popup(&mut self, mgr: &mut EventMgr, id: WindowId) {
            for i in 0..self.popups.len() {
                if id == self.popups[i].0 {
                    self.popups.remove(i);
                    mgr.send_action(TkAction::REGION_MOVED);
                    return;
                }
            }
        }

        fn resize_popups(&mut self, mgr: &mut SetRectMgr) {
            for i in 0..self.popups.len() {
                self.resize_popup(mgr, i);
            }
        }

        fn handle_closure(&mut self, mgr: &mut EventMgr) {
            if let Some((mut consume, update)) = self.drop.take() {
                consume(&mut self.w);
                mgr.trigger_update(update, 0);
            }
        }
    }
}

impl<W: Widget> Window<W> {
    /// Create
    pub fn new<T: ToString>(title: T, w: W) -> Window<W> {
        Window {
            core: Default::default(),
            restrict_dimensions: (true, false),
            title: title.to_string(),
            w,
            popups: Default::default(),
            drop: None,
            icon: None,
        }
    }

    /// Configure whether min/max dimensions are forced
    ///
    /// By default, the min size is enforced but not the max.
    pub fn set_restrict_dimensions(&mut self, min: bool, max: bool) {
        self.restrict_dimensions = (min, max);
    }

    /// Set a closure to be called on destruction, and return a future
    ///
    /// This is a convenience wrapper around [`Window::on_drop_boxed`].
    pub fn on_drop<T, F>(&mut self, consume: F) -> (Future<T>, UpdateHandle)
    where
        F: FnMut(&mut W) -> T + 'static,
    {
        self.on_drop_boxed(Box::new(consume))
    }

    /// Set a closure to be called on destruction, and return a future
    ///
    /// The closure `consume` is called when the window is destroyed, and yields
    /// a user-defined value. This value is returned through the returned
    /// [`Future`] object. In order to be notified when the future
    /// completes, its owner should call [`EventState::update_on_handle`] with the
    /// returned [`UpdateHandle`].
    ///
    /// Currently it is not possible for this closure to actually drop the
    /// widget, but it may alter its contents: it is the last method call on
    /// the widget. (TODO: given unsized rvalues (rfc#1909), the closure should
    /// consume self.)
    ///
    /// Panics if called more than once. In case the window is cloned, this
    /// closure is *not* inherited by the clone: in that case, `on_drop` may be
    /// called on the clone.
    pub fn on_drop_boxed<T>(
        &mut self,
        consume: Box<dyn FnMut(&mut W) -> T>,
    ) -> (Future<T>, UpdateHandle) {
        if self.drop.is_some() {
            panic!("Window::on_drop: attempt to set multiple drop closures");
        }
        let (future, finish) = Future::new_box_fnmut(consume);
        let update = UpdateHandle::new();
        self.drop = Some((finish, update));
        (future, update)
    }

    /// Set the window icon
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon;
    }

    /// Load the window icon from a path
    ///
    /// On error the icon is not set. The window may still be used.
    #[cfg(feature = "image")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "image")))]
    pub fn load_icon_from_path<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // TODO(opt): image loading could be de-duplicated with
        // DrawShared::image_from_path, but this may not be worthwhile.
        let im = image::io::Reader::open(path)?
            .with_guessed_format()?
            .decode()?
            .into_rgba8();
        let (w, h) = im.dimensions();
        self.icon = Some(Icon::from_rgba(im.into_vec(), w, h)?);
        Ok(())
    }
}

// This is like WidgetChildren::find, but returns a translated Rect.
fn find_rect(widget: &dyn Widget, id: WidgetId) -> Option<Rect> {
    match widget.find_child_index(&id) {
        Some(i) => {
            if let Some(w) = widget.get_child(i) {
                find_rect(w, id).map(|rect| rect - widget.translation())
            } else {
                None
            }
        }
        None if widget.eq_id(&id) => Some(widget.rect()),
        _ => None,
    }
}

impl<W: Widget> Window<W> {
    fn resize_popup(&mut self, mgr: &mut SetRectMgr, index: usize) {
        // Notation: p=point/coord, s=size, m=margin
        // r=window/root rect, c=anchor rect
        let r = self.core.rect;
        let popup = &mut self.popups[index].1;

        let c = find_rect(self.w.as_widget(), popup.parent.clone()).unwrap();
        let widget = self.w.find_widget_mut(&popup.id).unwrap();
        let mut cache = layout::SolveCache::find_constraints(widget, mgr.size_mgr());
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

        cache.apply_rect(widget, mgr, rect, false, true);
    }
}
