// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: Response type

/// Response type from [`Handler::handle`].
///
/// This type wraps [`Handler::Msg`] allowing both custom messages and toolkit
/// messages.
///
/// [`Handler::handle`]: super::Handler::handle
/// [`Handler::Msg`]: super::Handler::Msg
#[derive(Copy, Clone, Debug)]
#[must_use]
pub enum Response<M> {
    /// No action
    None,
    /// Custom message type
    Msg(M),
}

// Unfortunately we cannot write generic `From` / `TryFrom` impls
// due to trait coherence rules, so we impl `from` etc. directly.
impl<M> Response<M> {
    /// Map from one `Response` type to another
    ///
    /// Once Rust supports specialisation, this will likely be replaced with a
    /// `From` implementation.
    #[inline]
    pub fn from<N>(r: Response<N>) -> Self
    where
        M: From<N>,
    {
        r.try_into().unwrap_or_else(|msg| Response::Msg(M::from(msg)))
    }

    /// Map one `Response` type into another
    ///
    /// Once Rust supports specialisation, this will likely be redundant.
    #[inline]
    pub fn into<N>(self) -> Response<N>
    where
        N: From<M>,
    {
        Response::from(self)
    }

    /// Try mapping from one `Response` type to another, failing on `Msg`
    /// variant and returning the payload.
    #[inline]
    pub fn try_from<N>(r: Response<N>) -> Result<Self, N> {
        use Response::*;
        match r {
            None => Ok(None),
            Msg(m) => Err(m),
        }
    }

    /// Try mapping one `Response` type into another, failing on `Msg`
    /// variant and returning the payload.
    #[inline]
    pub fn try_into<N>(self) -> Result<Response<N>, M> {
        Response::try_from(self)
    }
}

impl<M> From<M> for Response<M> {
    fn from(msg: M) -> Self {
        Response::Msg(msg)
    }
}
