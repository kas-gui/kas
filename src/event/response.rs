// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: Response type

use super::{Event, VoidResponse};
use kas::geom::Rect;

/// Response type from [`Handler::handle`].
///
/// This type wraps [`Handler::Msg`] allowing both custom messages and toolkit
/// messages.
///
/// [`Handler::handle`]: super::Handler::handle
/// [`Handler::Msg`]: super::Handler::Msg
#[derive(Clone, Debug)]
#[must_use]
pub enum Response<M> {
    /// No action
    None,
    /// Unhandled input events get returned back up the widget tree
    Unhandled(Event),
    /// (Keyboard) focus has changed. This region should be made visible.
    Focus(Rect),
    /// Custom message type
    Msg(M),
}

// Unfortunately we cannot write generic `From` / `TryFrom` impls
// due to trait coherence rules, so we impl `from` etc. directly.
impl<M> Response<M> {
    /// True if variant is `None`
    #[inline]
    pub fn is_none(&self) -> bool {
        match self {
            &Response::None => true,
            _ => false,
        }
    }

    /// True if variant is `Unhandled`
    #[inline]
    pub fn is_unhandled(&self) -> bool {
        match self {
            &Response::Unhandled(_) => true,
            _ => false,
        }
    }

    /// True if variant is `Msg`
    #[inline]
    pub fn is_msg(&self) -> bool {
        match self {
            &Response::Msg(_) => true,
            _ => false,
        }
    }

    /// Map from one `Response` type to another
    ///
    /// Once Rust supports specialisation, this will likely be replaced with a
    /// `From` implementation.
    #[inline]
    pub fn from<N>(r: Response<N>) -> Self
    where
        N: Into<M>,
    {
        r.try_into().unwrap_or_else(|msg| Response::Msg(msg.into()))
    }

    /// Map one `Response` type into another
    ///
    /// Once Rust supports specialisation, this will likely be redundant.
    #[inline]
    pub fn into<N>(self) -> Response<N>
    where
        M: Into<N>,
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
            Unhandled(e) => Ok(Unhandled(e)),
            Focus(rect) => Ok(Focus(rect)),
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

impl VoidResponse {
    /// Convert a `Response<VoidMsg>` to another `Response`
    pub fn void_into<M>(self) -> Response<M> {
        self.try_into().unwrap_or(Response::None)
    }
}

impl<M> From<M> for Response<M> {
    fn from(msg: M) -> Self {
        Response::Msg(msg)
    }
}

impl<M> From<Option<M>> for Response<M> {
    fn from(msg: Option<M>) -> Self {
        match msg {
            Some(msg) => Response::Msg(msg),
            None => Response::None,
        }
    }
}
