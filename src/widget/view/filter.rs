// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter accessor

use super::Accessor;
use kas::conv::{Cast, Conv};
#[allow(unused)]
use kas::event::Manager;
use kas::event::UpdateHandle;
use std::fmt::Debug;

/// Filter accessor over another accessor
#[derive(Clone, Debug)]
pub struct FilterAccessor<I, T: Accessor<I>>
where
    I: Cast<u32> + Cast<usize> + Conv<u32> + Conv<usize> + Debug + 'static,
{
    data: T,
    view: Vec<u32>,
    update: UpdateHandle,
    _i: std::marker::PhantomData<I>,
}

impl<I, T: Accessor<I>> FilterAccessor<I, T>
where
    I: Cast<u32> + Cast<usize> + Conv<u32> + Conv<usize> + Debug + 'static,
{
    /// Construct, with all data hidden (filtered out)
    ///
    /// This is the fastest constructor.
    pub fn new_hidden(data: T) -> Self {
        let view = Vec::with_capacity(data.len().cast());
        FilterAccessor {
            data,
            view,
            update: UpdateHandle::new(),
            _i: Default::default(),
        }
    }

    /// Construct, with all data visible
    pub fn new_visible(data: T) -> Self {
        let mut x = Self::new_hidden(data);
        x.view.extend(0..x.data.len().cast());
        x
    }

    /// Update the filtered view
    ///
    /// An update should be triggered using the returned handle.
    pub fn update_filter<F: Fn(T::Item) -> bool>(&mut self, filter: F) -> UpdateHandle {
        self.view.clear();
        // TODO: is this slow?
        for i in 0..self.data.len().cast() {
            if filter(self.data.get(i.cast())) {
                self.view.push(i);
            }
        }
        self.update
    }
}

impl<I, T: Accessor<I>> Accessor<I> for FilterAccessor<I, T>
where
    I: Cast<u32> + Cast<usize> + Conv<u32> + Conv<usize> + Debug + 'static,
{
    type Item = T::Item;
    fn len(&self) -> I {
        self.view.len().cast()
    }
    fn get(&self, index: I) -> Self::Item {
        self.data.get(self.view[Cast::<usize>::cast(index)].cast())
    }
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.update)
    }
}
