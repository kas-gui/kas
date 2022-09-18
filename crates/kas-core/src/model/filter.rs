// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filters over data

mod filter_list;
pub use filter_list::FilteredList;

use crate::event::EventMgr;
use crate::model::*;
use std::cell::{Ref, RefCell};
use std::fmt::Debug;
use std::rc::Rc;

/// Types usable as a filter
pub trait Filter<T>: 'static {
    /// Returns true if the given item matches this filter
    fn matches(&self, item: &T) -> bool;
}

/// Filter: target contains self (case-sensitive string match)
#[derive(Debug, Default, Clone)]
pub struct ContainsString(Rc<RefCell<(String, u64)>>);

impl ContainsString {
    /// Construct with given string
    pub fn new<S: ToString>(s: S) -> Self {
        let data = RefCell::new((s.to_string(), 1));
        ContainsString(Rc::new(data))
    }
}
impl SharedData for ContainsString {
    type Key = ();
    type Item = String;
    type ItemRef<'b> = Ref<'b, String>;

    fn version(&self) -> u64 {
        self.0.borrow().1
    }

    fn contains_key(&self, _: &Self::Key) -> bool {
        true
    }
    fn borrow(&self, _: &Self::Key) -> Option<Self::ItemRef<'_>> {
        Some(Ref::map(self.0.borrow(), |tuple| &tuple.0))
    }

    fn update(&self, mgr: &mut EventMgr, _: &Self::Key, value: Self::Item) {
        let mut cell = self.0.borrow_mut();
        cell.0 = value;
        cell.1 += 1;
        mgr.update_all(0);
    }
}

impl<'a> Filter<&'a str> for ContainsString {
    fn matches(&self, item: &&str) -> bool {
        item.contains(&self.0.borrow().0)
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
pub struct ContainsCaseInsensitive(Rc<RefCell<(String, String, u64)>>);

impl ContainsCaseInsensitive {
    /// Construct with given string
    pub fn new<S: ToString>(s: S) -> Self {
        let s = s.to_string();
        let u = s.to_uppercase();
        let data = RefCell::new((s, u, 1));
        ContainsCaseInsensitive(Rc::new(data))
    }
}
impl SharedData for ContainsCaseInsensitive {
    type Key = ();
    type Item = String;
    type ItemRef<'b> = Ref<'b, String>;

    fn version(&self) -> u64 {
        self.0.borrow().2
    }

    fn contains_key(&self, _: &Self::Key) -> bool {
        true
    }
    fn borrow(&self, _: &Self::Key) -> Option<Self::ItemRef<'_>> {
        Some(Ref::map(self.0.borrow(), |tuple| &tuple.0))
    }

    fn update(&self, mgr: &mut EventMgr, _: &Self::Key, value: Self::Item) {
        let mut cell = self.0.borrow_mut();
        cell.0 = value;
        cell.1 = cell.0.to_uppercase();
        cell.2 += 1;
        mgr.update_all(0);
    }
}

impl<'a> Filter<&'a str> for ContainsCaseInsensitive {
    fn matches(&self, item: &&str) -> bool {
        item.to_string().to_uppercase().contains(&self.0.borrow().1)
    }
}
impl Filter<String> for ContainsCaseInsensitive {
    fn matches(&self, item: &String) -> bool {
        Filter::<&str>::matches(self, &item.as_str())
    }
}
