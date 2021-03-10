// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data types

use std::cell::RefCell;
use std::rc::Rc;

// for doc use
#[allow(unused)]
use kas::event::UpdateHandle;

/// A future value, pending UI operation
///
/// This is a simpler form of future than the [`std::future::Future`] trait,
/// relying on the UI event system for execution. It also does not come with a
/// waker; since calling into widget code is only possible through the event
/// system, an [`UpdateHandle`] should be used to wake the recipient.
#[derive(Debug)]
pub struct Future<T: 'static>(Rc<RefCell<Option<T>>>);
impl<T: 'static> Future<T> {
    /// Construct, given a boxed closure to produce the final value
    ///
    /// Returns the future and a `finish` closure to set the value when done.
    pub fn new_box_fnmut<U: 'static>(
        mut f: Box<dyn FnMut(&mut U) -> T>,
    ) -> (Self, Box<dyn FnMut(&mut U)>) {
        let target: Rc<RefCell<Option<T>>> = Default::default();
        let t2 = target.clone();
        let finish: Box<dyn FnMut(&mut U)> = Box::new(move |u| *t2.borrow_mut() = Some(f(u)));
        (Future(target), finish)
    }

    /// Check whether this is finished
    pub fn is_finished(&self) -> bool {
        Rc::strong_count(&self.0) == 1
    }

    /// Returns a result on completion
    ///
    /// It may be worth checking [`Future::is_finished`] before calling this method.
    pub fn try_finish(self) -> Result<T, Self> {
        Rc::try_unwrap(self.0)
            .map(|cell| {
                cell.into_inner()
                    .unwrap_or_else(|| panic!("Future finished without setting a value!"))
            })
            .map_err(|target| Future(target))
    }
}
