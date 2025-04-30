// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects

use kas::cast::Cast;
use kas::event::{ConfigCx, EventCx};
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
macro_rules! impl_1D {
    ($t:ty) => {
        impl DataKey for $t {
            fn make_id(&self, parent: &Id) -> Id {
                parent.make_child((*self).cast())
            }

            fn reconstruct_key(parent: &Id, child: &Id) -> Option<Self> {
                child.next_key_after(parent).map(|i| i.cast())
            }
        }
    };
}
impl_1D!(usize);
impl_1D!(u32);
#[cfg(target_pointer_width = "64")]
impl_1D!(u64);

macro_rules! impl_2D {
    ($t:ty) => {
        impl DataKey for ($t, $t) {
            fn make_id(&self, parent: &Id) -> Id {
                parent.make_child(self.0.cast()).make_child(self.1.cast())
            }

            fn reconstruct_key(parent: &Id, child: &Id) -> Option<Self> {
                let mut iter = child.iter_keys_after(parent);
                let col = iter.next().map(|i| i.cast());
                let row = iter.next().map(|i| i.cast());
                col.zip(row)
            }
        }
    };
}
impl_2D!(usize);
impl_2D!(u32);
#[cfg(target_pointer_width = "64")]
impl_2D!(u64);

/// Accessor for data
///
/// The trait covers access to data (especially important for data which must be retrieved from a
/// remote database or generated on demand). The implementation may optionally support filtering.
///
/// Type parameter `Index` is `usize` for `ListView` or `MatrixIndex` for `MatrixView`.
///
/// # Implementing `DataAccessor`
///
/// Data keys ([`Self::key`]) should always be independent of the search query
/// or filter. The key may simply be the input `index` if the `index` will
/// always correspond to a fixed data `Item`. This may not be the case if a
/// (variable) filter or query is used or if the items available through the
/// data set are not fixed.
///
/// ## Local fixed data sets
///
/// Accessing data stored within `self` or the input `data` type [`Self::Data`]
/// is the simplest case; it may be sufficient to implement the required methods
/// [`Self::len`], [`Self::key`] and [`Self::item`] only.
///
/// ## Dynamic local data sets
///
/// In case of a local data set which may change or where the query or filter
/// used changes, the method [`Self::update`] must be implemented and it must be
/// ensured that the widget's [`kas::Events::update`] method is called (the
/// latter will already be the case when the changing data/query/filter is
/// passed via input `data`).
///
/// The result of [`Self::len`] should be updated to return the number of
/// available elements. (TODO: this should not be required provided that the
/// available scroll range is provided or estimated somehow.)
///
/// ## Generated data
///
/// In some cases, data `Item`s may be generated on demand. This is problematic
/// since [`Self::item`] must return a *reference to* the data. The solution is
/// to generate this data using the [`Self::prepare_range`] method, caching
/// generated values within `self`. (It is assumed that such use cases are
/// relatively rare and/or simple, otherwise the return value may be changed to
/// `Option<std::borrow::Cow<Item>>`.)
///
/// Note that [`Self::prepare_range`] may be called frequently, thus (at risk of
/// premature optimization) it should not unnecessarily regenerate items on each
/// call. In case input data changes, [`Self::update`] will be called followed
/// by [`Self::prepare_range`].
///
/// If generation is slow, it should be performed asynchronously (see below)
/// so that all methods may return quickly.
///
/// ## Non-local data (`async`)
///
/// Method implementations should never block. For non-local data, this requires
/// the usage of `async` message handling; [`Self::update`],
/// [`Self::prepare_range`] and [`Self::handle_messages`] may all dispatch
/// `async` queries using `cx.send_async(id, QUERY)`.
/// The result will be received by [`Self::handle_messages`].
///
/// The number of available elements (if not known in advance) should be
/// requested by [`Self::update`] when required (note that the method may be
/// called frequently and without changes to input `data`). It is acceptable if
/// the result is updated asynchronously, nothing that [`Self::prepare_range`]
/// will be limited to the current result of [`Self::len`].
/// It is acceptable if not all indices less than `len` will return a valid key
/// through [`Self::key`], though simply returning a very large `len` will not
/// work well with scrollbars (TODO: better support for estimated lengths).
///
/// If data keys cannot be generated locally on demand they may be requested by
/// [`Self::update`] and/or [`Self::prepare_range`].
///
/// Data items should be requested through [`Self::prepare_range`] as required,
/// caching results locally when received by [`Self::handle_messages`]. It is up
/// to the implementation whether to continue caching items outside of the
/// latest requested `range`.
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
    ///
    /// To receive (async) messages with [`Self::handle_messages`], send to `id`
    /// using (for example) `cx.send_async(id, _)`.
    ///
    /// The default implementation does nothing.
    fn update(&mut self, cx: &mut ConfigCx, id: Id, data: &Self::Data) {
        let _ = (cx, id, data);
    }

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
    ///
    /// To receive (async) messages with [`Self::handle_messages`], send to `id`
    /// using (for example) `cx.send_async(id, _)`.
    ///
    /// The default implementation does nothing.
    fn prepare_range(&mut self, cx: &mut ConfigCx, id: Id, data: &Self::Data, range: Range<Index>) {
        let _ = (cx, id, data, range);
    }

    /// Handle an async message
    ///
    /// This method is called when a message is available, possibly the result
    /// of an asynchronous message sent through [`Self::update`] or
    /// [`Self::prepare_range`]. The implementation should
    /// [try_pop](EventCx::try_pop) messages of types sent by this trait impl
    /// but not messages of other types.
    ///
    /// To receive (async) messages with [`Self::handle_messages`], send to `id`
    /// using (for example) `cx.send_async(id, _)`.
    ///
    /// The default implementation does nothing.
    fn handle_messages(&mut self, cx: &mut EventCx, id: Id, data: &Self::Data) {
        let _ = (cx, id, data);
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
