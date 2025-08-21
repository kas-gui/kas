// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data generation (high-level) traits

use crate::{DataChanges, DataClerk, DataKey, DataLen, Token, TokenChanges};
use kas::Id;
use kas::event::ConfigCx;
#[allow(unused)] use kas::{Action, Events, Widget};
use std::fmt::Debug;
use std::marker::PhantomData;

/// Indicates whether an update to a [`DataGenerator`] has changed
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum GeneratorChanges {
    /// `None` indicates that no changes to the data has occurred.
    None,
    /// Indicates that [`DataGenerator::len`] may have changed but generated
    /// values have not changed.
    LenOnly,
    /// `Any` indicates that changes to the data set may have occurred.
    Any,
}

/// A generator for use with [`GeneratorClerk`]
///
/// This provides a substantially simpler interface than [`DataClerk`].
pub trait DataGenerator<Index> {
    /// Input data type (of parent widget)
    ///
    /// Data of this type is passed through the parent widget; see
    /// [`Widget::Data`] and the [`Events`] trait. This input data might be used
    /// to access a data set stored in another widget or to pass a query or
    /// filter into the `DataClerk`.
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

    /// Item type
    ///
    /// This is the generated type.
    type Item: Clone + Default;

    /// Update the generator
    ///
    /// This is called by [`kas::Events::update`]. It should update `self` as
    /// required reflecting possible data-changes and indicate through the
    /// returned [`GeneratorChanges`] value the updates required to tokens and
    /// views.
    ///
    /// Note: this method is called automatically when input data changes. When
    /// data owned or referenced by the `DataClerk` implementation is changed it
    /// may be necessary to explicitly update the view controller, e.g. using
    /// [`ConfigCx::update`] or [`Action::UPDATE`].
    ///
    /// This method may be called frequently and without changes to `data`.
    fn update(&mut self, data: &Self::Data) -> GeneratorChanges;

    /// Get the number of indexable items
    ///
    /// Scroll bars and the `index` values passed to [`Self::generate`] are
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

    /// Generate an item
    ///
    /// The `index` will be less than the result of the last call to
    /// [`Self::len`].
    fn generate(&self, data: &Self::Data, index: Index) -> Self::Item;
}

/// An implementation of [`DataClerk`] for data generators
pub struct GeneratorClerk<Index, G: DataGenerator<Index>> {
    g: G,
    _index: PhantomData<Index>,
}

impl<Index: Default, G: DataGenerator<Index>> GeneratorClerk<Index, G> {
    /// Construct a `GeneratorClerk`
    pub fn new(generator: G) -> Self {
        GeneratorClerk {
            g: generator,
            _index: PhantomData,
        }
    }
}

impl<Index: DataKey, G: DataGenerator<Index>> DataClerk<Index> for GeneratorClerk<Index, G> {
    type Data = G::Data;
    type Key = Index;
    type Item = G::Item;
    type Token = Token<Self::Key, Self::Item>;

    fn update(&mut self, _: &mut ConfigCx, _: Id, data: &Self::Data) -> DataChanges {
        match self.g.update(data) {
            GeneratorChanges::None => DataChanges::None,
            GeneratorChanges::LenOnly => DataChanges::NoPreparedItems,
            GeneratorChanges::Any => DataChanges::Any,
        }
    }

    fn len(&self, data: &Self::Data, lbound: Index) -> DataLen<Index> {
        self.g.len(data, lbound)
    }

    fn update_token(
        &self,
        data: &Self::Data,
        index: Index,
        token: &mut Option<Self::Token>,
    ) -> TokenChanges {
        if let Some(token) = token.as_ref()
            && token.key == index
        {
            return TokenChanges::None;
        }

        *token = Some(Token {
            key: index.clone(),
            item: self.g.generate(data, index),
        });
        TokenChanges::Any
    }

    fn item<'r>(&'r self, _: &'r Self::Data, token: &'r Self::Token) -> &'r Self::Item {
        &token.item
    }
}
