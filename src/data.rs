// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data types

use std::cell::RefCell;
use std::convert::TryFrom;
use std::fmt;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::u32;

use crate::geom::Rect;

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

/// Widget identifier
///
/// All widgets are assigned an identifier which is unique within the window.
/// This type may be tested for equality and order.
///
/// This type is small and cheap to copy. Internally it is "NonZero", thus
/// `Option<WidgetId>` is a free extension (requires no extra memory).
///
/// Identifiers are assigned when configured and when re-configured
/// (via [`kas::TkAction::RECONFIGURE`]). Since user-code is not notified of a
/// re-configure, user-code should not store a `WidgetId`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WidgetId(NonZeroU32);

impl WidgetId {
    pub(crate) const FIRST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(1) });
    const LAST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(u32::MAX) });

    pub(crate) fn next(self) -> Self {
        WidgetId(NonZeroU32::new(self.0.get() + 1).unwrap())
    }
}

impl TryFrom<u32> for WidgetId {
    type Error = ();
    fn try_from(x: u32) -> Result<WidgetId, ()> {
        NonZeroU32::new(x).map(|n| WidgetId(n)).ok_or(())
    }
}

impl TryFrom<u64> for WidgetId {
    type Error = ();
    fn try_from(x: u64) -> Result<WidgetId, ()> {
        if let Ok(x) = u32::try_from(x) {
            if let Some(nz) = NonZeroU32::new(x) {
                return Ok(WidgetId(nz));
            }
        }
        Err(())
    }
}

impl From<WidgetId> for u32 {
    #[inline]
    fn from(id: WidgetId) -> u32 {
        id.0.get()
    }
}

impl From<WidgetId> for u64 {
    #[inline]
    fn from(id: WidgetId) -> u64 {
        id.0.get().into()
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        WidgetId::LAST
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "#{}", self.0)
    }
}

#[test]
fn size_of_option_widget_id() {
    use std::mem::size_of;
    assert_eq!(size_of::<WidgetId>(), size_of::<Option<WidgetId>>());
}

/// Common widget data
///
/// All widgets should embed a `#[widget_core] core: CoreData` field.
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    pub rect: Rect,
    pub id: WidgetId,
    pub disabled: bool,
}
