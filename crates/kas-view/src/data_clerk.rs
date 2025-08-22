// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects

use kas::Id;
use kas::cast::Cast;
use kas::event::{ConfigCx, EventCx};
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

/// Indicates whether an update to a [`DataClerk`] changes any keys or values
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum DataChanges {
    /// `None` indicates that no changes to the data set occurred.
    None,
    /// `NoPrepared` indicates that changes to the data set may have occurred,
    /// but that for all indices in the `range` last passed to
    /// [`DataClerk::prepare_range`] the index-key mappings ([`DataClerk::key`]
    /// results) and key-value mappings ([`DataClerk::item`] results) remain
    /// unchanged.
    NoPrepared,
    /// `NoPreparedKeys` indicates that changes to the data set may have
    /// occurred, but that for all indices in the `range` last passed to
    /// [`DataClerk::prepare_range`] the index-key mappings ([`DataClerk::key`]
    /// results) remain unchanged.
    NoPreparedKeys,
    /// `Any` indicates that changes to the data set may have occurred.
    Any,
}

/// Data access manager
///
/// A `DataClerk` manages access to a data set, using an `Index` type specified by
/// the [view controller](crate#view-controller).
///
/// Instances are expected to provide access to a subset of data elements (as
/// specified by [`Self::prepare_range`]), either via direct access or via an
/// internal cache.
///
/// Each data item must have a unique key of type [`Self::Key`]. Where the data
/// view is affected by a variable filter or query the index-item relationship
/// may vary; the key-item relationship must not vary.
///
/// # Implementing `DataClerk`
///
/// ## Local fixed data sets
///
/// If the data set is immutable and stored within `self` or within input data
/// (see type [`Self::Data`]) it is sufficient to implement only [`Self::len`],
/// [`Self::key`] and [`Self::item`]. All these methods take a `data` parameter
/// thus enabling direct referencing of data items from either `self` or input
/// data.
///
/// ## Dynamic local data sets
///
/// In case of a local data set which may change or where the query or filter
/// used changes, the method [`Self::update`] must be implemented and it must be
/// ensured that the widget's [`kas::Events::update`] method is called (the
/// latter will already be the case when the changing data/query/filter is
/// passed via input `data`).
///
/// The result of [`Self::len`] and/or [`Self::min_len`] should be updated to
/// return the number of available elements.
///
/// As above, it may be possible to implement [`Self::item`] by referencing the
/// data directly.
///
/// ## Generated data
///
/// In some cases, data `Item`s may be generated on demand. This is not
/// *directly* supported since [`Self::item`] must return a *reference to* the
/// data (and is expected to be very fast). Instead, a cache of (at least) the
/// currently visible items must be generated by [`Self::prepare_range`].
///
/// Note that [`Self::prepare_range`] may be called frequently, thus (at risk of
/// premature optimization) it should not unnecessarily regenerate items on each
/// call. The `range.len()` will rarely change and frequently this range will
/// only move a little from the previously-visible range, thus it may be
/// sensible to use a
/// [circular buffer](https://en.wikipedia.org/wiki/Circular_buffer) for the
/// cache; elements may then be indexed by `index % range.len()`.
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
/// If the number of available elements (i.e. [`Self::len`]) is not known in
/// advance then [`Self::update`] should request this. Note that
/// [`Self::prepare_range`] will never attempt to access elements beyond the
/// current result of [`Self::len`] and that it is safe (but possibly
/// undesirable) for [`Self::len`] to report too large a value.
/// (TODO: rework this; the main thing affected is the length of scrollbars.)
///
/// If data keys cannot be generated locally on demand they may be requested by
/// [`Self::update`] and/or [`Self::prepare_range`].
///
/// Data items should be requested through [`Self::prepare_range`] as required,
/// caching results locally when received by [`Self::handle_messages`]. It is up
/// to the implementation whether to continue caching items outside of the
/// latest requested `range`.
pub trait DataClerk<Index> {
    /// Input data type (of parent widget)
    ///
    /// This input data might provide access to the data set or might be used
    /// for some other purpose (such as passing in a filter from an input field)
    /// or might not be used at all.
    ///
    /// Note that it is not currently possible to pass in references to multiple
    /// data items (such as an external data set and a filter) via `Data`. This
    /// would require use of Generic Associated Types (GATs), not only here but
    /// also in the [`Widget`](kas::Widget) trait; alas, GATs are not (yet)
    /// compatible with dyn traits and Kas requires use of `dyn Widget`. Instead
    /// one can share the data set (e.g. `Rc<RefCell<DataSet>>`) or store within
    /// the `DataClerk` using the `clerk` / `clerk_mut` methods to access; in
    /// both cases it may be necessary to update the view controller explicitly
    /// (e.g. `cx.update(list.as_node(&input))`) after the data set changes.
    type Data;

    /// Key type
    ///
    /// All data items should have a stable key so that data items may be
    /// tracked through changing queries.
    type Key: DataKey;

    /// Item type
    ///
    /// `&Item` is passed to child view widgets as input data.
    type Item;

    /// Update the query
    ///
    /// This is called by [`kas::Events::update`]. It may be called frequently
    /// and without changes to `data` and should use `async` execution for
    /// expensive or slow calculations.
    ///
    /// This method should perform any updates required to adjust [`Self::len`]
    /// and [`Self::min_len`] or arrange for these properties to be updated
    /// asynchronously.
    ///
    /// To receive (async) messages with [`Self::handle_messages`], send to `id`
    /// using (for example) `cx.send_async(id, _)`.
    ///
    /// There is no default implementation since it is unknown what
    /// [`DataChanges`] might be needed.
    fn update(&mut self, cx: &mut ConfigCx, id: Id, data: &Self::Data) -> DataChanges;

    /// Get the number of indexable items, if known
    ///
    /// If known, the result should be one larger than the largest `index`
    /// yielding a result from [`Self::key`]. This number may therefore be
    /// affected by input `data` such as filters.
    ///
    /// The result may change after a call to [`Self::update`] due to changes in
    /// the data set query or filter. The result should not depend on `range`.
    ///
    /// This method may return [`None`], in which case [`Self::min_len`] must be
    /// implemented instead. This will affect the appearance of scroll bars.
    fn len(&self, data: &Self::Data) -> Option<Index>;

    /// Get a lower bound on the number of indexable items
    ///
    /// This method is only called when [`Self::len`] returns [`None`].
    ///
    /// If the return value is less than `expected`, then scrolling and querying
    /// will be limited to indices less than the return value. If the return
    /// value is at least `expected`, then scrolling and item querying will be
    /// unimpeded.
    ///
    /// In case [`Self::len`] returns [`None`], this value is used to size
    /// scroll bar grips.
    fn min_len(&self, data: &Self::Data, expected: Index) -> Index {
        let _ = expected;
        self.len(data).unwrap()
    }

    /// Prepare a range
    ///
    /// This method is called any time that the accessed range might be expected
    /// to change. It may be called frequently and without changes to `range`
    /// and should use `async` execution for expensive or slow calculations.
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
    /// This method is called when a message is available. Such messages may be
    /// taken using [`EventCx::try_pop`]. It is not required that all messages
    /// be handled (some may be intended for other recipients).
    ///
    /// When `key.is_some()`, the message's source is a view widget over the
    /// data item with this key. This allows a custom view widget to send a
    /// custom message, possibly affecting the data set.
    ///
    /// When `key.is_none()` the message may be from the view controller or may
    /// be the result of an asynchronous message sent through [`Self::update`]
    /// or [`Self::prepare_range`].
    ///
    /// To receive (async) messages with [`Self::handle_messages`], send to `id`
    /// using (for example) `cx.send_async(id, _)`.
    ///
    /// The default implementation does nothing.
    fn handle_messages(
        &mut self,
        cx: &mut EventCx,
        id: Id,
        data: &Self::Data,
        key: Option<&Self::Key>,
    ) -> DataChanges {
        let _ = (cx, id, data, key);
        DataChanges::None
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
    /// data set, this method should not return hidden keys. The implementation
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
