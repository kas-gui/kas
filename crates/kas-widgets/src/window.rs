// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use kas::prelude::*;
use kas::Future;
use kas::Icon;

impl_scope! {
    /// The main instantiation of the [`Window`] trait.
    #[autoimpl(Clone ignore self.drop where W: Clone)]
    #[autoimpl(Debug ignore self.drop, self.icon)]
    #[widget(layout = self.w;)]
    pub struct Window<W: Widget + 'static> {
        core: widget_core!(),
        restrict_dimensions: (bool, bool),
        title: String,
        #[widget]
        w: W,
        drop: Option<(Box<dyn FnMut(&mut W)>, UpdateHandle)>,
        icon: Option<Icon>,
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
