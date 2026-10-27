// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data transfer types
//!
//! This module is an extension of [`winit::data_transfer`].

use std::ops::ControlFlow;

pub use winit::data_transfer::*;

/// A data transfer supporting only [`TypeHint::Plaintext`].
#[derive(Clone, Debug)]
pub struct PlainUtf8Transfer(String);
impl DataTransfer for PlainUtf8Transfer {
    fn for_each_available_type<'a>(
        &'a self,
        f: &mut dyn FnMut(&'a dyn TransferType) -> ControlFlow<()>,
    ) {
        let _ = f(&TypeHint::Plaintext);
    }
}
impl DataTransferSend for PlainUtf8Transfer {
    fn data_for_type(&self, type_: &dyn TransferType) -> Option<SendData> {
        type_
            .matches(&TypeHint::Plaintext)
            .then_some(self.0.clone().into())
    }
}

impl From<String> for PlainUtf8Transfer {
    fn from(content: String) -> Self {
        PlainUtf8Transfer(content)
    }
}
