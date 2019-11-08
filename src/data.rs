// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data types

use std::fmt;
use std::num::NonZeroU32;
use std::u32;

use crate::event::VirtualKeyCode;
use crate::{geom::Rect, Core};

/// Widget identifier
///
/// All widgets within a window are assigned a unique numeric identifier. This
/// type may be tested for equality and order.
///
/// Note: identifiers are first assigned when a window is instantiated by the
/// toolkit.
#[derive(Debug, Clone, Copy, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct WidgetId(NonZeroU32);

impl WidgetId {
    #[doc(hidden)]
    pub const FIRST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(1) });
    const LAST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(u32::MAX) });

    #[doc(hidden)]
    pub(crate) fn next(self) -> Self {
        WidgetId(NonZeroU32::new(self.0.get() + 1).unwrap())
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        WidgetId::LAST
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}

/// Common widget data
///
/// All widgets should embed a `core: CoreData` field in order to implement the
/// [`Core`] macro.
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    pub rect: Rect,
    pub id: WidgetId,
    // variable-length list; None may not preceed Some(_)
    keys: [Option<VirtualKeyCode>; 4],
}

impl CoreData {
    /// Set shortcut keys
    pub fn set_keys(&mut self, keys: &[VirtualKeyCode]) {
        if keys.len() > self.keys.len() {
            panic!(
                "CoreData::set_keys: found {} keys; max supported is {}",
                keys.len(),
                self.keys.len()
            );
        }
        for (source, dest) in keys.iter().copied().zip(&mut self.keys) {
            *dest = Some(source);
        }
        for dest in &mut self.keys[keys.len()..] {
            *dest = None;
        }
    }

    /// Get shortcut keys
    pub fn keys<'a>(&'a self) -> impl Iterator<Item = VirtualKeyCode> + 'a {
        self.keys
            .iter()
            .take_while(|x| x.is_some())
            .fuse()
            .map(|x| x.unwrap())
    }
}

impl Core for CoreData {
    #[inline]
    fn core_data(&self) -> &CoreData {
        self
    }

    #[inline]
    fn core_data_mut(&mut self) -> &mut CoreData {
        self
    }
}
