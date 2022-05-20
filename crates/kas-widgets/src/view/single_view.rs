// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{driver, Driver};
use kas::prelude::*;
use kas::updatable::SingleData;

impl_scope! {
    /// Single view widget
    ///
    /// This widget supports a view over a shared data item.
    ///
    /// The shared data type `T` must support [`SingleData`].
    /// One may use [`kas::updatable::SharedRc`]
    /// or a custom shared data type.
    ///
    /// The driver `V` must implement [`Driver`], with data type
    /// `<T as SingleData>::Item`. Several implementations are available in the
    /// [`driver`] module or a custom implementation may be used.
    ///
    /// # Messages
    ///
    /// When a child pushes a message, the [`SingleData::handle_message`] method is
    /// called.
    #[autoimpl(Debug ignore self.view)]
    #[derive(Clone)]
    #[widget{
        layout = self.child;
    }]
    pub struct SingleView<
        T: SingleData,
        V: Driver<T::Item> = driver::DefaultView,
    > {
        core: widget_core!(),
        view: V,
        data: T,
        data_ver: u64,
        #[widget]
        child: V::Widget,
    }

    impl Default for Self
    where
        T: Default,
        V: Default,
    {
        fn default() -> Self {
            Self::new(T::default())
        }
    }
    impl Self where V: Default {
        /// Construct a new instance
        pub fn new(data: T) -> Self {
            Self::new_with_driver(<V as Default>::default(), data)
        }
    }
    impl Self {
        /// Construct a new instance with explicit view
        pub fn new_with_driver(view: V, data: T) -> Self {
            let child = view.make();
            let data_ver = data.version();
            SingleView {
                core: Default::default(),
                view,
                data,
                data_ver,
                child,
            }
        }

        /// Access the data object
        pub fn data(&self) -> &T {
            &self.data
        }

        /// Access the data object (mut)
        pub fn data_mut(&mut self) -> &mut T {
            &mut self.data
        }

        /// Get a copy of the shared value
        pub fn get_value(&self) -> T::Item {
            self.data.get_cloned()
        }

        /// Set shared data
        ///
        /// This method updates the shared data, if supported (see
        /// [`SingleData::update`]). Other widgets sharing this data are notified
        /// of the update, if data is changed.
        pub fn set_value(&self, mgr: &mut EventMgr, data: T::Item) {
            self.data.update(mgr, data);
        }

        /// Update shared data
        ///
        /// This is purely a convenience method over [`SingleView::set_value`].
        /// It notifies other widgets of updates to the shared data.
        pub fn update_value<F: Fn(T::Item) -> T::Item>(&self, mgr: &mut EventMgr, f: F) {
            self.set_value(mgr, f(self.get_value()));
        }
    }

    impl Widget for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            // We set data now, after child is configured
            *mgr |= self.view.set(&mut self.child, self.data.get_cloned());
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Update { .. } => {
                    let data_ver = self.data.version();
                    if data_ver > self.data_ver {
                        let value = self.data.get_cloned();
                        *mgr |= self.view.set(&mut self.child, value);
                        self.data_ver = data_ver;
                    }
                    Response::Used
                }
                _ => Response::Unused,
            }
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            self.data.handle_message(mgr);
        }
    }
}
