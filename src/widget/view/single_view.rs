// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::driver::{self, Driver};
use kas::data::{SingleData, UpdatableHandler};
use kas::prelude::*;
use std::fmt::{self};

/// Single view widget
///
/// This widget supports a view over a shared data item.
///
/// The shared data type `T` must support [`SingleData`] and
/// [`UpdatableHandler`], the latter with key type `()` and message type
/// matching the widget's message. One may use [`kas::data::SharedRc`] or a
/// custom shared data type.
///
/// The driver `V` must implement [`Driver`], with data type
/// `<T as SingleData>::Item`. Several implementations are available in the
/// [`driver`] module or a custom implementation may be used.
#[derive(Clone, Widget)]
#[widget(config=noauto)]
#[layout(single)]
#[handler(handle=noauto, send=noauto)]
pub struct SingleView<
    T: SingleData + UpdatableHandler<(), V::Msg> + 'static,
    V: Driver<T::Item> = driver::Default,
> {
    #[widget_core]
    core: CoreData,
    view: V,
    data: T,
    #[widget]
    child: V::Widget,
}

impl<
        T: SingleData + UpdatableHandler<(), V::Msg> + 'static + Default,
        V: Driver<T::Item> + Default,
    > Default for SingleView<T, V>
{
    fn default() -> Self {
        Self::new(T::default())
    }
}
impl<T: SingleData + UpdatableHandler<(), V::Msg> + 'static, V: Driver<T::Item> + Default>
    SingleView<T, V>
{
    /// Construct a new instance
    pub fn new(data: T) -> Self {
        Self::new_with_view(<V as Default>::default(), data)
    }
}
impl<T: SingleData + UpdatableHandler<(), V::Msg> + 'static, V: Driver<T::Item>> SingleView<T, V> {
    /// Construct a new instance with explicit view
    pub fn new_with_view(view: V, data: T) -> Self {
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

impl<T: SingleData + UpdatableHandler<(), V::Msg> + 'static, V: Driver<T::Item>> WidgetConfig
    for SingleView<T, V>
{
    fn configure(&mut self, mgr: &mut Manager) {
        self.data.enable_recursive_updates(mgr);
        if let Some(handle) = self.data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
    }
}

impl<T: SingleData + UpdatableHandler<(), V::Msg> + 'static, V: Driver<T::Item>> Handler
    for SingleView<T, V>
{
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

impl<T: SingleData + UpdatableHandler<(), V::Msg> + 'static, V: Driver<T::Item>> SendEvent
    for SingleView<T, V>
{
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled;
        }

        if id < self.id() {
            let r = self.child.send(mgr, id, event);
            match r {
                Response::Msg(ref msg) => {
                    if let Some(handle) = self.data.handle(&(), &msg) {
                        mgr.trigger_update(handle, 0);
                    }
                }
                _ => (),
            }
            r
        } else {
            debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
            self.handle(mgr, event)
        }
    }
}

impl<T: SingleData + UpdatableHandler<(), V::Msg> + 'static, V: Driver<T::Item>> fmt::Debug
    for SingleView<T, V>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SingleView {{ core: {:?}, data: {:?}, child: {:?} }}",
            self.core, self.data, self.child,
        )
    }
}
