// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Core widget types

#[cfg(feature = "accesskit")] mod accesskit;
mod collection;
mod data;
mod layout;
mod node;
mod scroll_traits;
mod tile;
mod widget;
mod widget_id;

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub mod impls;

#[cfg(feature = "accesskit")]
pub use accesskit::AccessKitCx;
pub use collection::{CellCollection, Collection};
pub use data::*;
pub use layout::*;
pub use node::Node;
pub use scroll_traits::*;
pub use tile::*;
pub use widget::*;
pub use widget_id::*;
