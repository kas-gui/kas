// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data generation (high-level) traits

use super::{AsyncClerk, Clerk, DataChanges, DataClerk, DataKey, Token, TokenChanges};
use kas::Id;
use kas::event::ConfigCx;
#[allow(unused)] use kas::{Action, Events, Widget};
use std::fmt::Debug;
use std::ops::Range;

/// Indicates whether an update to a generator changes any available data
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub enum GeneratorChanges<Index> {
    /// `None` indicates that no changes to the data has occurred.
    None,
    /// Indicates that [`Clerk::len`] may have changed but generated
    /// values have not changed.
    LenOnly,
    /// Indicates that items in the given range may require an update.
    /// The `generate` method will be called for each index in the
    /// intersection of the given range with the visible data range.
    Range(Range<Index>),
    /// `Any` indicates that changes to the data set may have occurred.
    Any,
}

/// Interface for generators over indexed data
pub trait IndexedGenerator<Index>: Clerk<Index, Item: Clone + Default + PartialEq> {
    /// Update the generator
    ///
    /// This is called by [`kas::Events::update`]. It should update `self` as
    /// required reflecting possible data-changes and indicate through the
    /// returned [`GeneratorChanges`] value the updates required to tokens and
    /// views.
    ///
    /// Note: this method is called automatically when input data changes. When
    /// data owned or referenced by the `IndexedGenerator` implementation is
    /// changed it may be necessary to explicitly update the view controller,
    /// e.g. using [`ConfigCx::update`] or [`Action::UPDATE`].
    ///
    /// This method may be called frequently and without changes to `data`.
    fn update(&mut self, data: &Self::Data) -> GeneratorChanges<Index>;

    /// Generate an item
    fn generate(&self, data: &Self::Data, index: Index) -> Self::Item;
}

/// Interface for generators over keyed data
pub trait KeyedGenerator<Index>: Clerk<Index, Item: Clone + Default + PartialEq> {
    /// Key type
    ///
    /// All data items should have a stable key so that data items may be
    /// tracked through changing queries. This allows focus and selection to
    /// correctly track items when the data query or filter changes.
    ///
    /// Where the query is fixed, this can just be the `Index` type.
    type Key: DataKey;

    /// Update the generator
    ///
    /// This is called by [`kas::Events::update`]. It should update `self` as
    /// required reflecting possible data-changes and indicate through the
    /// returned [`GeneratorChanges`] value the updates required to tokens and
    /// views.
    ///
    /// Note: this method is called automatically when input data changes. When
    /// data owned or referenced by the `KeyedGenerator` implementation is
    /// changed it may be necessary to explicitly update the view controller,
    /// e.g. using [`ConfigCx::update`] or [`Action::UPDATE`].
    ///
    /// This method may be called frequently and without changes to `data`.
    fn update(&mut self, data: &Self::Data) -> GeneratorChanges<Index>;

    /// Get a key for a given `index`, if available
    ///
    /// This method should be fast since it may be called repeatedly.
    /// This method is only called for `index` less than the result of
    /// [`Clerk::len`].
    ///
    /// This may return `None` even when `index` is within the query's `range`
    /// since data may be sparse; in this case the view widget at this `index`
    /// is hidden.
    fn key(&self, data: &Self::Data, index: Index) -> Option<Self::Key>;

    /// Generate an item
    ///
    /// The `key` will be the result of [`Self::key`] for an `index` less than
    /// [`Clerk::len`].
    fn generate(&self, data: &Self::Data, key: &Self::Key) -> Self::Item;
}

impl<Index: DataKey, G: IndexedGenerator<Index>> KeyedGenerator<Index> for G {
    type Key = Index;

    fn update(&mut self, data: &Self::Data) -> GeneratorChanges<Index> {
        self.update(data)
    }

    fn key(&self, _: &Self::Data, index: Index) -> Option<Self::Key> {
        Some(index)
    }

    fn generate(&self, data: &Self::Data, key: &Self::Key) -> Self::Item {
        self.generate(data, key.clone())
    }
}

impl<Index, G: KeyedGenerator<Index>> AsyncClerk<Index> for G {
    type Key = G::Key;

    fn update(
        &mut self,
        _: &mut ConfigCx,
        _: Id,
        _: Range<Index>,
        data: &Self::Data,
    ) -> DataChanges<Index> {
        match self.update(data) {
            GeneratorChanges::None => DataChanges::None,
            GeneratorChanges::LenOnly => DataChanges::NoPreparedItems,
            GeneratorChanges::Range(range) => DataChanges::Range(range),
            GeneratorChanges::Any => DataChanges::Any,
        }
    }
}

impl<Index, G: KeyedGenerator<Index>> DataClerk<Index> for G {
    type Token = Token<Self::Key, Self::Item>;

    fn update_token(
        &self,
        data: &Self::Data,
        index: Index,
        update_item: bool,
        token: &mut Option<Self::Token>,
    ) -> TokenChanges {
        let Some(key) = self.key(data, index) else {
            *token = None;
            return TokenChanges::None;
        };

        if !update_item
            && let Some(token) = token.as_mut()
            && token.key == key
        {
            return TokenChanges::None;
        }

        let item = self.generate(data, &key);
        let mut changes = TokenChanges::Any;

        if let Some(token) = token.as_mut()
            && token.key == key
        {
            if token.item == item {
                return TokenChanges::None;
            } else {
                changes = TokenChanges::SameKey;
            }
        }

        *token = Some(Token { key, item });
        changes
    }

    #[inline]
    fn item<'r>(&'r self, _: &'r Self::Data, token: &'r Self::Token) -> &'r Self::Item {
        &token.item
    }
}
