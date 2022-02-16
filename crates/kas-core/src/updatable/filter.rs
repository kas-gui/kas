// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filters over data

use crate::event::{UpdateHandle, VoidMsg};
use crate::updatable::*;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// Types usable as a filter
pub trait Filter<T>: Debug + 'static {
    /// Returns true if the given item matches this filter
    // TODO: once Accessor::get returns a reference, this should take item: &T where T: ?Sized
    fn matches(&self, item: T) -> bool;
}

/// Filter: target contains self (case-sensitive string match)
#[derive(Debug, Default, Clone)]
pub struct ContainsString(Rc<(UpdateHandle, RefCell<(String, u64)>)>);

impl ContainsString {
    /// Construct with given string
    pub fn new<S: ToString>(s: S) -> Self {
        let handle = UpdateHandle::new();
        let data = RefCell::new((s.to_string(), 1));
        ContainsString(Rc::new((handle, data)))
    }
}
impl Updatable<(), String> for ContainsString {
    fn handle(&self, _: &(), msg: &String) -> bool {
        self.update(msg.clone())
    }
}
impl Updatable<(), VoidMsg> for ContainsString {
    fn handle(&self, _: &(), _: &VoidMsg) -> bool {
        false
    }
}
impl SingleData for ContainsString {
    type Item = String;
    fn version(&self) -> u64 {
        (self.0).1.borrow().1
    }
    fn get_cloned(&self) -> Self::Item {
        (self.0).1.borrow().0.to_owned()
    }
    fn update(&self, value: Self::Item) -> bool {
        let mut cell = (self.0).1.borrow_mut();
        cell.0 = value;
        cell.1 += 1;
        true
    }
}
impl SingleDataMut for ContainsString {
    fn set(&mut self, value: Self::Item) {
        let mut cell = (self.0).1.borrow_mut();
        cell.0 = value;
        cell.1 += 1;
    }
}

impl<'a> Filter<&'a str> for ContainsString {
    fn matches(&self, item: &str) -> bool {
        item.contains(&self.get_cloned())
    }
}
impl Filter<String> for ContainsString {
    fn matches(&self, item: String) -> bool {
        Filter::<&str>::matches(self, &item)
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
pub struct ContainsCaseInsensitive(Rc<(UpdateHandle, RefCell<(String, String, u64)>)>);

impl ContainsCaseInsensitive {
    /// Construct with given string
    pub fn new<S: ToString>(s: S) -> Self {
        let handle = UpdateHandle::new();
        let s = s.to_string();
        let u = s.to_uppercase();
        let data = RefCell::new((s, u, 1));
        ContainsCaseInsensitive(Rc::new((handle, data)))
    }
}
impl Updatable<(), String> for ContainsCaseInsensitive {
    fn handle(&self, _: &(), msg: &String) -> bool {
        self.update(msg.clone())
    }
}
impl Updatable<(), VoidMsg> for ContainsCaseInsensitive {
    fn handle(&self, _: &(), _: &VoidMsg) -> bool {
        false
    }
}
impl SingleData for ContainsCaseInsensitive {
    type Item = String;
    fn version(&self) -> u64 {
        (self.0).1.borrow().2
    }
    fn get_cloned(&self) -> Self::Item {
        (self.0).1.borrow().0.clone()
    }
    fn update(&self, value: Self::Item) -> bool {
        let mut cell = (self.0).1.borrow_mut();
        cell.0 = value;
        cell.1 = cell.0.to_uppercase();
        cell.2 += 1;
        true
    }
}
impl SingleDataMut for ContainsCaseInsensitive {
    fn set(&mut self, value: Self::Item) {
        let mut cell = (self.0).1.borrow_mut();
        cell.0 = value;
        cell.1 = cell.0.to_uppercase();
        cell.2 += 1;
    }
}

impl<'a> Filter<&'a str> for ContainsCaseInsensitive {
    fn matches(&self, item: &str) -> bool {
        Filter::<String>::matches(self, item.to_string())
    }
}
impl Filter<String> for ContainsCaseInsensitive {
    fn matches(&self, item: String) -> bool {
        item.to_uppercase().contains(&(self.0).1.borrow().1)
    }
}
