// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filters over data

mod filter_list;
pub use filter_list::*;

/// Ability to set filter
pub trait FilterValue: Default + 'static {
    type Value: std::fmt::Debug;
    fn set_filter(&mut self, value: Self::Value);
}

/// Types usable as a filter
pub trait Filter<T>: FilterValue {
    /// Returns true if the given item matches this filter
    fn matches(&self, item: &T) -> bool;
}

/// Filter: target contains self (case-sensitive string match)
#[derive(Debug, Default, Clone)]
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

impl<'a> Filter<&'a str> for ContainsString {
    fn matches(&self, item: &&str) -> bool {
        item.contains(&self.0)
    }
}
impl Filter<String> for ContainsString {
    fn matches(&self, item: &String) -> bool {
        Filter::<&str>::matches(self, &item.as_str())
    }
}

/// Filter: target contains self (case-insensitive string match)
///
// Note: the implemented method of caseless matching is not unicode compliant,
// however works in most cases (by converting both the source and the target to
// upper case). See [question on StackOverflow].
//
// [question on StackOverflow]: https://stackoverflow.com/questions/47298336/case-insensitive-string-matching-in-rust
#[derive(Debug, Default, Clone)]
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

impl<'a> Filter<&'a str> for ContainsCaseInsensitive {
    fn matches(&self, item: &&str) -> bool {
        item.to_string().to_uppercase().contains(&self.0)
    }
}
impl Filter<String> for ContainsCaseInsensitive {
    fn matches(&self, item: &String) -> bool {
        Filter::<&str>::matches(self, &item.as_str())
    }
}
