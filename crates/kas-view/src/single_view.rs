// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view controller

use super::{driver, Driver};
use kas::model::SingleData;
#[allow(unused)]
use kas::model::{SharedData, SharedDataMut, SharedRc};
use kas::prelude::*;
use std::borrow::Borrow;

impl_scope! {
    /// View controller for 0D data (singe item)
    ///
    /// This widget supports a view over a single shared data item.
    ///
    /// The shared data type `T` must support [`SingleData`].
    /// One may use [`SharedRc`] or a custom shared data type.
    ///
    /// The driver `V` must implement [`Driver`] over `T`.
    /// The default driver is [`driver::View`]; others are available in the
    /// [`driver`] module or [`Driver`] may be implemented directly.
    ///
    /// # Messages
    ///
    /// When a view widget pushes a message, [`Driver::on_message`] is called.
    #[derive(Clone)]
    #[widget{
        Data = <V::Widget as Widget>::Data;
        layout = self.child;
    }]
    pub struct SingleView<T: SingleData, V: Driver<T::Item, T> = driver::View> {
        core: widget_core!(),
        driver: V,
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
    impl Self
    where
        V: Default,
    {
        /// Construct a new instance
        pub fn new(data: T) -> Self {
            Self::new_with_driver(<V as Default>::default(), data)
        }
    }
    impl Self {
        /// Construct a new instance with explicit driver
        pub fn new_with_driver(driver: V, data: T) -> Self {
            let child = driver.make();
            let data_ver = data.version();
            SingleView {
                core: Default::default(),
                driver,
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

        /// Borrow a reference to the shared value
        pub fn borrow_value(&self) -> Option<impl Borrow<T::Item> + '_> {
            self.data.borrow(&())
        }

        /// Get a copy of the shared value
        pub fn get_value(&self) -> T::Item {
            self.data.get_cloned(&()).unwrap()
        }

        /// Set shared data
        ///
        /// This method updates the shared data, if supported (see
        /// [`SharedDataMut::borrow_mut`]). Other widgets sharing this data
        /// are notified of the update, if data is changed.
        pub fn set_value(&self, mgr: &mut EventMgr, data: T::Item)
        where
            T: SharedDataMut,
        {
            self.data.set(mgr, &(), data);
        }

        /// Update shared data
        ///
        /// This method updates the shared data, if supported (see
        /// [`SharedDataMut::with_ref_mut`]). Other widgets sharing this data
        /// are notified of the update, if data is changed.
        pub fn update_value<U>(
            &self,
            mgr: &mut EventMgr,
            f: impl FnOnce(&mut T::Item) -> U,
        ) -> Option<U>
        where
            T: SharedDataMut,
        {
            self.data.with_ref_mut(mgr, &(), f)
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let mut rules = self.child.size_rules(size_mgr, axis);
            rules.set_stretch(rules.stretch().max(Stretch::Low));
            rules
        }
    }

    impl Events for Self {
        fn configure(&mut self, _: &Self::Data, mgr: &mut ConfigMgr) {
            // We set data now, after child is configured
            let item = self.data.borrow(&()).unwrap();
            *mgr |= self.driver.set(&mut self.child, &(), item.borrow());
        }

        fn handle_event(&mut self, _: &Self::Data, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Update { .. } => {
                    let data_ver = self.data.version();
                    if data_ver > self.data_ver {
                        let item = self.data.borrow(&()).unwrap();
                        *mgr |= self.driver.set(&mut self.child, &(), item.borrow());
                        self.data_ver = data_ver;
                    }
                    Response::Used
                }
                _ => Response::Unused,
            }
        }

        fn handle_message(&mut self, _: &Self::Data, mgr: &mut EventMgr) {
            self.driver
                .on_message(mgr, &mut self.child, &self.data, &());
        }
    }
}
