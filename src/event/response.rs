// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling
//!
//! Event handling uses *event* messages, passed from the parent into a widget,
//! with responses passed back to the parent. This model is simpler than that
//! commonly used by GUI frameworks: widgets do not need a pointer to their
//! parent and any result is pushed back up the call stack. The model allows
//! type-safety while allowing user-defined result types.

/// Response type from [`Handler::handle`].
///
/// This type wraps [`Handler::Msg`] allowing both custom messages and toolkit
/// messages.
///
/// [`Handler::handle`]: super::Handler::handle
/// [`Handler::Msg`]: super::Handler::Msg
#[derive(Copy, Clone, Debug)]
pub enum Response<M> {
    /// No action
    None,
    /// Custom message type
    Msg(M),
}

// Unfortunately we cannot write generic `From` / `TryFrom` impls
// due to trait coherence rules, so we impl `from` etc. directly.
impl<M> Response<M> {
    /// Convert
    ///
    /// Once Rust supports specialisation, this will likely be replaced with a
    /// `From` implementation.
    #[inline]
    pub fn from<N>(r: Response<N>) -> Self
    where
        M: From<N>,
    {
        r.map_into(|msg| Response::Msg(M::from(msg)))
    }

    /// Convert
    ///
    /// Once Rust supports specialisation, this will likely be redundant.
    #[inline]
    pub fn into<N>(self) -> Response<N>
    where
        N: From<M>,
    {
        Response::from(self)
    }

    /// Convert from a `Response<()>`
    ///
    /// Once Rust supports specialisation, this will likely be replaced with a
    /// `From` implementation.
    #[inline]
    pub fn from_(r: Response<()>) -> Self {
        r.map_into(|_| Response::None)
    }

    /// Try converting, failing on `Msg` variant
    #[inline]
    pub fn try_from<N>(r: Response<N>) -> Result<Self, N> {
        use Response::*;
        match r {
            None => Ok(None),
            Msg(m) => Err(m),
        }
    }

    /// Try converting, failing on `Msg` variant
    #[inline]
    pub fn try_into<N>(self) -> Result<Response<N>, M> {
        Response::try_from(self)
    }

    /// Convert, applying map function on `Msg` variant
    #[inline]
    pub fn map_into<N, F: FnOnce(M) -> Response<N>>(self, op: F) -> Response<N> {
        Response::try_from(self).unwrap_or_else(op)
    }
}

impl Response<()> {
    /// Convert
    ///
    /// Once Rust supports specialisation, this will likely be removed.
    #[inline]
    pub fn into_<N>(self) -> Response<N> {
        Response::from_(self)
    }
}

impl<M> From<M> for Response<M> {
    fn from(msg: M) -> Self {
        Response::Msg(msg)
    }
}
