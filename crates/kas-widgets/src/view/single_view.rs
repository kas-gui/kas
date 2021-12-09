// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{driver, Driver};
use kas::prelude::*;
use kas::updatable::{SingleData, UpdatableHandler};
use UpdatableHandler as UpdHandler;

widget! {
    /// Single view widget
    ///
    /// This widget supports a view over a shared data item.
    ///
    /// The shared data type `T` must support [`SingleData`] and
    /// [`UpdatableHandler`], the latter with key type `()` and message type
    /// matching the widget's message. One may use [`kas::updatable::SharedRc`]
    /// or a custom shared data type.
    ///
    /// The driver `V` must implement [`Driver`], with data type
    /// `<T as SingleData>::Item`. Several implementations are available in the
    /// [`driver`] module or a custom implementation may be used.
    #[autoimpl(Debug skip view)]
    #[derive(Clone)]
    #[widget{
        layout = single;
    }]
    pub struct SingleView<
        T: SingleData + UpdHandler<(), V::Msg> + 'static,
        V: Driver<T::Item> = driver::Default,
    > {
        #[widget_core]
        core: CoreData,
        view: V,
        data: T,
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
            let mut child = view.new();
            let _ = view.set(&mut child, data.get_cloned());
            SingleView {
                core: Default::default(),
                view,
                data,
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
        pub fn set_value(&self, mgr: &mut Manager, data: T::Item) {
            if let Some(handle) = self.data.update(data) {
                mgr.trigger_update(handle, 0);
            }
        }

        /// Update shared data
        ///
        /// This is purely a convenience method over [`SingleView::set_value`].
        /// It notifies other widgets of updates to the shared data.
        pub fn update_value<F: Fn(T::Item) -> T::Item>(&self, mgr: &mut Manager, f: F) {
            self.set_value(mgr, f(self.get_value()));
        }
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut Manager) {
            if let Some(handle) = self.data.update_handle() {
                mgr.update_on_handle(handle, self.id());
            }
        }
    }

    impl Handler for Self {
        type Msg = <V::Widget as Handler>::Msg;
        fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
            match event {
                Event::HandleUpdate { .. } => {
                    let value = self.data.get_cloned();
                    *mgr |= self.view.set(&mut self.child, value);
                    Response::Update
                }
                _ => Response::Unhandled,
            }
        }
    }

    impl SendEvent for Self {
        fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if self.is_disabled() {
                return Response::Unhandled;
            }

            if id == self.id() {
                self.handle(mgr, event)
            } else {
                let r = self.child.send(mgr, id, event);
                if matches!(&r, Response::Update | Response::Msg(_)) {
                    if let Some(value) = self.view.get(&self.child) {
                        if let Some(handle) = self.data.update(value) {
                            mgr.trigger_update(handle, 0);
                        }
                    }
                }
                if let Response::Msg(ref msg) = &r {
                    log::trace!(
                        "Received by {} from {}: {:?}",
                        self.id(),
                        id,
                        kas::util::TryFormat(&msg)
                    );
                    if let Some(handle) = self.data.handle(&(), msg) {
                        mgr.trigger_update(handle, 0);
                    }
                }
                r
            }
        }
    }
}
