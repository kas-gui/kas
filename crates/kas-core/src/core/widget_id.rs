// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget identifiers

use crate::cast::Conv;
use std::fmt;
use std::iter::once;
use std::num::NonZeroU64;
use std::sync::Mutex;

/// Widget identifier
///
/// All widgets are assigned an identifier which is unique within the window.
/// This type may be tested for equality and order.
///
/// This type is small and cheap to copy. Internally it is "NonZero", thus
/// `Option<WidgetId>` is a free extension (requires no extra memory).
///
/// Identifiers are assigned when configured and when re-configured
/// (via [`crate::TkAction::RECONFIGURE`]). Since user-code is not notified of a
/// re-configure, user-code should not store a `WidgetId`.
#[derive(Clone, Copy, Debug, Hash, Eq)]
pub struct WidgetId(NonZeroU64);

/// The first byte (head) controls interpretation of the rest
const MASK_HEAD: u64 = 0xFF_00_0000_0000_0000;
const MASK_REST: u64 = !MASK_HEAD;

/// `(x & MASK_HEAD) == USE_DB`: rest is index in DB
const USE_DB: u64 = 0x01_00_0000_0000_0000;

impl WidgetId {
    /// Identifier of the window
    pub(crate) const ROOT: WidgetId = WidgetId(unsafe { NonZeroU64::new_unchecked(USE_DB) });

    const INVALID: WidgetId = WidgetId(unsafe { NonZeroU64::new_unchecked(MASK_REST) });

    /// Returns true if `self` equals `id` or if `id` is a descendant of `self`
    pub fn is_ancestor_of(self, id: Self) -> bool {
        let self_id = self.0.get();
        let child_id = id.0.get();
        match (self_id & MASK_HEAD, child_id & MASK_HEAD) {
            (USE_DB, USE_DB) => {
                let db = DB.lock().unwrap();

                let self_i = usize::conv(self_id & MASK_REST);
                let child_i = usize::conv(child_id & MASK_REST);

                let result = db[child_i].starts_with(&db[self_i]);
                result
            }
            _ => false,
        }
    }

    /// Get index of `child` relative to `self`
    ///
    /// Returns `None` if `child` is not a descendant of `self`.
    pub fn index_of_child(self, child: Self) -> Option<usize> {
        let self_id = self.0.get();
        let child_id = child.0.get();
        match (self_id & MASK_HEAD, child_id & MASK_HEAD) {
            (USE_DB, USE_DB) => {
                let db = DB.lock().unwrap();

                let self_i = usize::conv(self_id & MASK_REST);
                let child_i = usize::conv(child_id & MASK_REST);

                let result = if db[child_i].starts_with(&db[self_i]) {
                    let self_len = db[self_i].len();
                    db[child_i][self_len..].iter().next().cloned()
                } else {
                    None
                };
                result
            }
            _ => None,
        }
    }

    /// Make an identifier for the child with the given `index`
    ///
    /// Note: this is not a getter method. Calling multiple times with the same
    /// `index` may or may not return the same value!
    pub fn make_child(self, index: usize) -> Self {
        let self_id = self.0.get();
        match self_id & MASK_HEAD {
            USE_DB => {
                // We can assume the child must also use the DB
                let mut db = DB.lock().unwrap();

                let i = usize::conv(self_id & MASK_REST);
                let path = db[i].iter().cloned().chain(once(index)).collect();

                let id = u64::conv(db.len());
                // We can quite safely assume this:
                debug_assert_eq!(id & MASK_HEAD, 0);
                let id = id & MASK_REST;

                db.push(path);

                WidgetId(NonZeroU64::new(USE_DB | id).unwrap())
            }
            _ => panic!("WidgetId::make_child: cannot make child of {}", self),
        }
    }

    /// Convert `Option<WidgetId>` to `u64`
    pub fn opt_to_u64(id: Option<WidgetId>) -> u64 {
        match id {
            None => 0,
            Some(id) => id.into(),
        }
    }

    /// Convert `u64` to `Option<WidgetId>`
    ///
    /// This always "succeeds", though the result may not identify any widget.
    pub fn opt_from_u64(n: u64) -> Option<WidgetId> {
        NonZeroU64::new(n).map(|nz| WidgetId(nz))
    }
}

impl std::cmp::PartialEq for WidgetId {
    fn eq(&self, rhs: &Self) -> bool {
        let self_id = self.0.get();
        let rhs_id = rhs.0.get();
        match (self_id & MASK_HEAD, rhs_id & MASK_HEAD) {
            (USE_DB, USE_DB) => {
                let db = DB.lock().unwrap();

                let self_i = usize::conv(self_id & MASK_REST);
                let child_i = usize::conv(rhs_id & MASK_REST);
                db[self_i] == db[child_i]
            }
            _ => self_id == rhs_id,
        }
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
        WidgetId::INVALID
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let self_id = self.0.get();
        match self_id & MASK_HEAD {
            USE_DB => write!(f, "DB#{}", self_id & MASK_REST),
            _ => todo!(),
        }
    }
}

lazy_static::lazy_static! {
    static ref DB: Mutex<Vec<Vec<usize>>> = Mutex::new(vec![vec![]]);
}

#[test]
fn size_of_option_widget_id() {
    use std::mem::size_of;
    assert_eq!(size_of::<WidgetId>(), size_of::<Option<WidgetId>>());
}
