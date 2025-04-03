// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects

use kas::event::EventCx;
use kas::Id;
#[allow(unused)] // doc links
use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::Range;

/// Bounds on the key type
///
/// This type should be small, easy to copy, and without internal mutability.
pub trait DataKey: Clone + Debug + Default + PartialEq + Eq + 'static {
    /// Make an [`Id`] for a key
    ///
    /// The result must be distinct from `parent` and a descendant of `parent`
    /// (use [`Id::make_child`] for this, optionally more than once).
    fn make_id(&self, parent: &Id) -> Id;

    /// Reconstruct a key from an [`Id`]
    ///
    /// Where `child` is the output of [`Self::make_id`] for the same `parent`
    /// *or any [`Id`] descended from that*, this should return a copy of
    /// the `key` passed to `make_id`.
    ///
    /// See: [`Id::next_key_after`], [`Id::iter_keys_after`]
    fn reconstruct_key(parent: &Id, child: &Id) -> Option<Self>;
}

impl DataKey for () {
    fn make_id(&self, parent: &Id) -> Id {
        // We need a distinct child, so use index 0
        parent.make_child(0)
    }

    fn reconstruct_key(parent: &Id, child: &Id) -> Option<Self> {
        if child.next_key_after(parent) == Some(0) {
            Some(())
        } else {
            None
        }
    }
}

// NOTE: we cannot use this blanket impl without specialisation / negative impls
// impl<Key: Cast<usize> + Clone + Debug + PartialEq + Eq + 'static> DataKey for Key
impl DataKey for usize {
    fn make_id(&self, parent: &Id) -> Id {
        parent.make_child(*self)
    }

    fn reconstruct_key(parent: &Id, child: &Id) -> Option<Self> {
        child.next_key_after(parent)
    }
}

impl DataKey for (usize, usize) {
    fn make_id(&self, parent: &Id) -> Id {
        parent.make_child(self.0).make_child(self.1)
    }

    fn reconstruct_key(parent: &Id, child: &Id) -> Option<Self> {
        let mut iter = child.iter_keys_after(parent);
        let col = iter.next();
        let row = iter.next();
        col.zip(row)
    }
}

/// Accessor for data
///
/// The trait covers access to data (especially important for data which must be retrieved from a
/// remote database or generated on demand). The implementation may optionally support filtering.
///
/// Type parameter `Index` is `usize` for `ListView` or `(usize, usize)` for `MatrixView`.
pub trait DataAccessor<Index> {
    /// Input data type (of parent widget)
    type Data;

    /// Key type
    ///
    /// All data items should have a stable key so that data items may be
    /// tracked through changing filters.
    type Key: DataKey;

    /// Item type
    ///
    /// `&Item` is passed to child view widgets as input data.
    type Item: Clone;

    /// Update the query
    ///
    /// This is called by [`kas::Events::update`]. It may be called frequently
    /// and without changes to `data` and should use `async` execution for
    /// expensive or slow calculations.
    ///
    /// This method should perform any updates required to adjust [`Self::len`].
    fn update(&mut self, data: &Self::Data);

    /// Get the total number of items
    ///
    /// The result should be one larger than the largest `index` yielding a
    /// result from [`Self::key`]. This number may therefore be affected by
    /// input `data` such as filters.
    ///
    /// The result may change after a call to [`Self::update`] due to changes in
    /// the data set query or filter. The result should not depend on `range`.
    ///
    /// TODO: revise how scrolling works and remove this method or make optional
    /// since the result is sometimes expensive to calculate.
    fn len(&self, data: &Self::Data) -> Index;

    /// Prepare a range
    ///
    /// This method is called after [`Self::update`] and any time that the
    /// accessed range might be expected to change. It may be called frequently
    /// and without changes to `range` and should use `async` execution for
    /// expensive or slow calculations.
    ///
    /// It should prepare for [`Self::key`] to be called on the given `range`
    /// and [`Self::item`] to be called for the returnable keys. These methods
    /// may be called immediately after this method, though it is acceptable for
    /// them to return [`None`] until keys / data items are available.
    ///
    /// This method may update cached keys and items asynchronously (i.e. after
    /// this method returns); in this case it should prioritise updating of keys
    /// over items.
    fn prepare_range(&mut self, data: &Self::Data, range: Range<Index>);

    /// Handle an async message
    ///
    /// This method is called when a message is available, possibly the result
    /// of an asynchronous message sent through [`Self::update`] or
    /// [`Self::prepare_range`]. The implementation should
    /// [try_pop](EventCx::try_pop) messages of types sent by this trait impl
    /// but not messages of other types.
    ///
    /// Default implementation: do nothing.
    fn handle_messages(&mut self, cx: &mut EventCx, data: &Self::Data) {
        let _ = (cx, data);
    }

    /// Get a key for a given `index`, if available
    ///
    /// This method should be fast since it may be called repeatedly.
    /// The method may be called for each `index` in the given `range` after
    /// calls to [`Self::update`].
    ///
    /// This may return `None` even when `index` is within the query's `range`
    /// since data may be sparse or still loading (async).
    ///
    /// In case the implementation applies some type of filter to an underlying
    /// dataset, this method should not return hidden keys. The implementation
    /// may either return [`None`] (resulting in empty list entries) or remap
    /// indices such that hidden keys are skipped over.
    fn key(&self, data: &Self::Data, index: Index) -> Option<Self::Key>;

    /// Get a data item, if available
    ///
    /// This method should be fast since it may be called repeatedly.
    ///
    /// This may return `None` while data is still loading (async). The view
    /// widget may display a loading animation in this case.
    fn item<'r>(&'r self, data: &'r Self::Data, key: &'r Self::Key) -> Option<&'r Self::Item>;
}
