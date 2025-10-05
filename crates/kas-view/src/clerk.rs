// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data clerks
//!
//! # Interfaces
//!
//! A clerk manages a view or query over a data set using an `Index` type
//! specified by the [view controller](crate#view-controller). For
//! [`ListView`](crate::ListView), `Index = usize`.
//!
//! A clerk must implement [`Clerk`] and one of the following traits.
//!
//! ## Data generators
//!
//! This simplest interface available is [`DataGenerator`]. This is appropriate
//! for data items which are generated (or cloned) on demand.
//!
//! ## Async data access
//!
//! The lowest level interface is [`DataClerk`]. This is designed to facilitate
//! async access to data using a local cache.

#[allow(unused)] use crate::SelectionMsg;
use kas::Id;
use kas::cast::Cast;
use kas::event::{ConfigCx, EventCx};
#[allow(unused)] use kas::{Action, Events, Widget};
use std::borrow::Borrow;
use std::fmt::Debug;
use std::ops::Range;

mod generator;
pub use generator::*;

/// A pair which may be borrowed over the first item
#[derive(Debug, Default)]
pub struct Token<K, I> {
    pub key: K,
    pub item: I,
}

impl<K, I> Token<K, I> {
    /// Construct
    #[inline]
    pub fn new(key: K, item: I) -> Self {
        Token { key, item }
    }
}

impl<K, I> Borrow<K> for Token<K, I> {
    fn borrow(&self) -> &K {
        &self.key
    }
}

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
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub enum DataChanges<Index> {
    /// Indicates that no changes to the data set occurred.
    None,
    /// Indicates that changes to the data set may have occurred, but that
    /// [`DataClerk::update_token`] and [`DataClerk::item`] results are
    /// unchanged for the `view_range`.
    NoPreparedItems,
    /// Indicates that tokens for the given range may require an update
    /// and/or that items for the given range have changed.
    /// [`DataClerk::update_token`] will be called for each index in the
    /// intersection of the given range with the `view_range`.
    Range(Range<Index>),
    /// `Any` indicates that changes to the data set may have occurred.
    Any,
}

/// Return value of [`DataClerk::update_token`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum TokenChanges {
    /// `None` indicates that no changes to the token occurred.
    None,
    /// `SameKey` indicates that while the token still represents the same key,
    /// the associated data item may have changed.
    SameKey,
    /// `Any` indicates that the data key (and item) may have changed.
    Any,
}

impl TokenChanges {
    pub(crate) fn key(self) -> bool {
        self == TokenChanges::Any
    }

    pub(crate) fn item(self) -> bool {
        self != TokenChanges::None
    }
}

/// Result of [`Self::len`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataLen<Index> {
    /// Length is known and specified exactly
    Known(Index),
    /// A lower bound on length is specified
    LBound(Index),
}

impl<Index: Copy> DataLen<Index> {
    /// Returns the length payload (known or lower bound)
    #[inline]
    pub fn len(&self) -> Index {
        match self {
            DataLen::Known(len) => *len,
            DataLen::LBound(len) => *len,
        }
    }

    /// Returns true if a known length given
    #[inline]
    pub fn is_known(&self) -> bool {
        matches!(self, DataLen::Known(_))
    }
}

/// Common functionality of all clerks
pub trait Clerk<Index> {
    /// Input data type (of parent widget)
    ///
    /// Data of this type is passed through the parent widget; see
    /// [`Widget::Data`] and the [`Events`] trait. This input data might be used
    /// to access a data set stored in another widget or to pass a query or
    /// filter into the `Clerk`.
    ///
    /// Note that it is not currently possible to pass in references to multiple
    /// data items (such as an external data set and a filter) via `Data`. This
    /// would require use of Generic Associated Types (GATs), not only here but
    /// also in the [`Widget`] trait; alas, GATs are not (yet)
    /// compatible with dyn traits and Kas requires use of `dyn Widget`. Instead
    /// one can share the data set (e.g. `Rc<RefCell<DataSet>>`) or store within
    /// the `Clerk` using the `clerk` / `clerk_mut` methods to access; in
    /// both cases it may be necessary to update the view controller explicitly
    /// (e.g. `cx.update(list.as_node(&input))`) after the data set changes.
    type Data;

    /// Item type
    ///
    /// `&Item` is passed to child view widgets as input data.
    type Item;

    /// Get an upper bound on length, if any
    ///
    /// Scroll bars and the `view_range` are
    /// limited by the result of this method.
    ///
    /// Where the data set size is a known fixed `len` (or unfixed but with
    /// maximum `len <= lbound`), this method should return
    /// <code>[DataLen::Known][](len)</code>.
    ///
    /// Where the data set size is unknown (or unfixed and greater than
    /// `lbound`), this method should return
    /// <code>[DataLen::LBound][](lbound)</code>.
    ///
    /// `lbound` is set to allow scrolling a little beyond the current view
    /// position (i.e. a little larger than the last prepared `range.end`).
    fn len(&self, data: &Self::Data, lbound: Index) -> DataLen<Index>;

    /// Get a mock data item for sizing purposes
    ///
    /// This method is called if no data items are available when initially
    /// sizing the view. If an item is returned, then a mock view widget is
    /// created using this data in order to determine size requirements.
    ///
    /// The default implementation returns `None`.
    fn mock_item(&self, data: &Self::Data) -> Option<Self::Item> {
        let _ = data;
        None
    }
}

/// Data access manager
///
/// A `DataClerk` manages access to a data set, using an `Index` type specified by
/// the [view controller](crate#view-controller).
///
/// In simpler cases it is sufficient to implement only required methods.
pub trait DataClerk<Index>: Clerk<Index> {
    /// Key type
    ///
    /// All data items should have a stable key so that data items may be
    /// tracked through changing queries. This allows focus and selection to
    /// correctly track items when the data query or filter changes.
    ///
    /// Where the query is fixed, this can just be the `Index` type.
    type Key: DataKey;

    /// Token type
    ///
    /// Each view widget is stored with a corresponding token set by
    /// [`Self::update_token`].
    ///
    /// Often this will either be [`Self::Key`] or
    /// <code>[Token]&lt;[Self::Key], [Self::Item](Clerk::Item)&gt;</code>.
    type Token: Borrow<Self::Key>;

    /// Update the clerk
    ///
    /// This is called by [`kas::Events::update`]. It should update `self` as
    /// required reflecting possible data-changes and indicate through the
    /// returned [`DataChanges`] value the updates required to tokens and views.
    ///
    /// Data items within `view_range` may be visible.
    ///
    /// Note: this method is called automatically when input data changes. When
    /// data owned or referenced by the `DataClerk` implementation is changed it
    /// may be necessary to explicitly update the view controller, e.g. using
    /// [`ConfigCx::update`] or [`Action::UPDATE`].
    ///
    /// This method may be called frequently and without changes to `data`.
    /// It is expected to be fast and non-blocking. Asynchronous updates to
    /// `self` are possible using [`Self::handle_messages`].
    fn update(
        &mut self,
        cx: &mut ConfigCx,
        id: Id,
        view_range: Range<Index>,
        data: &Self::Data,
    ) -> DataChanges<Index>;

    /// Prepare a range
    ///
    /// This method is called prior to [`Self::update_token`] over the indices
    /// in `range`. If data is to be loaded from a remote source or computed in
    /// a worker thread, it should be done so from here using `async` worker(s)
    /// (see [`Self::handle_messages`]).
    ///
    /// Data items within `view_range` may be visible.
    ///
    /// The passed `range` may be a subset of the `view_range` but does
    /// not exceed it; pre-emptive loading is left to the implementation.
    /// This method may be called frequently and without changes to `range`, and
    /// is expected to be fast and non-blocking.
    ///
    /// The default implementation does nothing.
    fn prepare_range(
        &mut self,
        cx: &mut ConfigCx,
        id: Id,
        view_range: Range<Index>,
        data: &Self::Data,
        range: Range<Index>,
    ) {
        let _ = (cx, id, view_range, data, range);
    }

    /// Handle an async message
    ///
    /// This method is called when a message is available. Such messages may be
    /// taken using [`EventCx::try_pop`]. Messages may be received from:
    ///
    /// -   The view widget for `key` when `opt_key = Some(key)`.
    /// -   [`SelectionMsg`] may be received from the view controller.
    /// -   [`Self::update`], [`Self::prepare_range`] and this method may send
    ///     `async` messages using `cx.send_async(controller.id(), SomeMessage { .. })`.
    ///
    /// Data items within `view_range` may be visible.
    ///
    /// The default implementation does nothing.
    fn handle_messages(
        &mut self,
        cx: &mut EventCx,
        id: Id,
        view_range: Range<Index>,
        data: &Self::Data,
        opt_key: Option<Self::Key>,
    ) -> DataChanges<Index> {
        let _ = (cx, id, view_range, data, opt_key);
        DataChanges::None
    }

    /// Update a token for the given `index`
    ///
    /// This method is called after [`Self::prepare_range`] for each `index` in
    /// the prepared `range` in order to prepare a to prepare a token for each
    /// item (see [`Self::item`]).
    ///
    /// The input `token` (if any) may or may not correspond to the given
    /// `index`. This method should prepare it as follows:
    ///
    /// -   If no item is currently available for `index`, set `*token = None`
    ///     and return any value of [`TokenChanges`].
    /// -   Otherwise, if the input `token` is `None` or corresponds to a
    ///     different `index`, replace `token` and report [`TokenChanges::Any`].
    /// -   Otherwise, if then token depends on (caches) the data item and
    ///     `update_item`, the token should be updated. The method should report
    ///     [`TokenChanges::SameKey`] when the token has changed.
    /// -   Finally (if none of the above), report [`TokenChanges::None`].
    ///
    /// This method should be fast since it may be called repeatedly. Slow and
    /// blocking operations should be run asynchronously from
    /// [`Self::prepare_range`] using an internal cache.
    fn update_token(
        &self,
        data: &Self::Data,
        index: Index,
        update_item: bool,
        token: &mut Option<Self::Token>,
    ) -> TokenChanges;

    /// Get the data item for the given `token`
    ///
    /// Data cannot be generated by this method but it can be generated by
    /// [`Self::update_token`] and cached within a [`Token`]
    /// (see [`Self::Token`]).
    ///
    /// A token is expected to be able to resolve an item. Since [`Self::Token`]
    /// does not support `Clone` it is known that items can be evicted from
    /// storage when their token is replaced.
    ///
    /// This method should be fast since it may be called repeatedly.
    fn item<'r>(&'r self, data: &'r Self::Data, token: &'r Self::Token) -> &'r Self::Item;
}
