// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filters over data

use kas::event::EventCx;
use kas_widgets::{EditField, EditGuard};
use std::fmt::Debug;

/// Ability to set filter
pub trait FilterValue: Default + 'static {
    type Value: std::fmt::Debug;

    /// Update the filter
    fn set_filter(&mut self, value: Self::Value);
}

/// Types usable as a filter
pub trait Filter<T: ?Sized>: FilterValue {
    /// Returns true if the given item matches this filter
    fn matches(&self, item: &T) -> bool;
}

/// Filter: target contains self (case-sensitive string match)
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContainsString(String);

impl ContainsString {
    /// Construct with empty text
    pub fn new() -> Self {
        ContainsString(String::new())
    }
}

impl FilterValue for ContainsString {
    type Value = String;
    fn set_filter(&mut self, value: String) {
        self.0 = value;
    }
}

impl Filter<str> for ContainsString {
    fn matches(&self, item: &str) -> bool {
        item.contains(&self.0)
    }
}
impl Filter<String> for ContainsString {
    fn matches(&self, item: &String) -> bool {
        Filter::<str>::matches(self, item.as_str())
    }
}

/// Filter: target contains self (case-insensitive string match)
///
// Note: the implemented method of caseless matching is not unicode compliant,
// however works in most cases (by converting both the source and the target to
// upper case). See [question on StackOverflow].
//
// [question on StackOverflow]: https://stackoverflow.com/questions/47298336/case-insensitive-string-matching-in-rust
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContainsCaseInsensitive(String);

impl ContainsCaseInsensitive {
    /// Construct with empty text
    pub fn new() -> Self {
        ContainsCaseInsensitive(String::new())
    }
}

impl FilterValue for ContainsCaseInsensitive {
    type Value = String;
    fn set_filter(&mut self, value: String) {
        self.0 = value.to_uppercase();
    }
}

impl Filter<str> for ContainsCaseInsensitive {
    fn matches(&self, item: &str) -> bool {
        item.to_string().to_uppercase().contains(&self.0)
    }
}
impl Filter<String> for ContainsCaseInsensitive {
    fn matches(&self, item: &String) -> bool {
        Filter::<str>::matches(self, item.as_str())
    }
}

#[derive(Debug, Default)]
pub struct SetFilter<T: Debug>(pub T);

/// An [`EditGuard`] which sends a [`SetFilter`] message on every change
///
/// This may be used for search-as-you-type.
pub struct KeystrokeGuard;
impl EditGuard for KeystrokeGuard {
    type Data = ();

    fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Self::Data) {
        cx.push(SetFilter(edit.as_str().to_string()));
    }
}

/// An [`EditGuard`] which sends a [`SetFilter`] message on activate and focus loss
///
/// This may be used for search-as-you-type.
pub struct AflGuard;
impl EditGuard for AflGuard {
    type Data = ();

    #[inline]
    fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Self::Data) {
        cx.push(SetFilter(edit.as_str().to_string()));
    }
}
