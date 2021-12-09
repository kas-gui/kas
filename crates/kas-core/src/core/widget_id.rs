// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget identifiers

use std::fmt;
use std::num::NonZeroU64;

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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WidgetId(NonZeroU64);

impl WidgetId {
    pub(crate) const FIRST: WidgetId = WidgetId(unsafe { NonZeroU64::new_unchecked(1) });
    const LAST: WidgetId = WidgetId(unsafe { NonZeroU64::new_unchecked(u64::MAX) });

    pub(crate) fn next(self) -> Self {
        WidgetId(NonZeroU64::new(self.0.get() + 1).unwrap())
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
