// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: Response type

use super::VoidResponse;
use crate::geom::{Offset, Rect};

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
    /// Event was unused
    ///
    /// Unused events may be used by a parent/ancestor widget or passed to
    /// another handler until used.
    Unused,
    /// Event is used, no other result
    ///
    /// All variants besides `Unused` indicate that the event was used. This
    /// variant is used when no further action happens.
    Used,
    /// Pan scrollable regions by the given delta
    ///
    /// This may be returned to scroll the closest scrollable ancestor region.
    /// This region should attempt to scroll self by this offset, then, if all
    /// the offset was used, return `Response::Scrolled`, otherwise return
    /// `Response::Pan(d)` with the unused offset `d`.
    ///
    /// With the usual scroll offset conventions, this delta must be subtracted
    /// from the scroll offset.
    Pan(Offset),
    /// Notify that an inner region scrolled
    Scrolled,
    /// (Keyboard) focus has changed
    ///
    /// This region (in the child's coordinate space) should be made visible.
    Focus(Rect),
    /// Widget wishes to be selected (or have selection status toggled)
    Select,
    /// Custom message type
    ///
    /// This signals a (possible) update to the widget's data, while passing a
    /// data payload to the parent widget.
    Msg(M),
}

// Unfortunately we cannot write generic `From` / `TryFrom` impls
// due to trait coherence rules, so we impl `from` etc. directly.
impl<M> Response<M> {
    /// Construct `None` or `Msg(msg)`
    #[inline]
    pub fn used_or_msg(opt_msg: Option<M>) -> Self {
        match opt_msg {
            None => Response::Used,
            Some(msg) => Response::Msg(msg),
        }
    }

    /// True if variant is `Used`
    #[inline]
    pub fn is_used(&self) -> bool {
        matches!(self, Response::Used)
    }

    /// True if variant is `Unused`
    #[inline]
    pub fn is_unused(&self) -> bool {
        matches!(self, Response::Unused)
    }

    /// True if variant is `Msg`
    #[inline]
    pub fn is_msg(&self) -> bool {
        matches!(self, Response::Msg(_))
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
            Unused => Ok(Unused),
            Used => Ok(Used),
            Pan(delta) => Ok(Pan(delta)),
            Scrolled => Ok(Scrolled),
            Focus(rect) => Ok(Focus(rect)),
            Select => Ok(Select),
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
        self.try_into().unwrap_or(Response::Used)
    }
}

impl<M> From<M> for Response<M> {
    fn from(msg: M) -> Self {
        Response::Msg(msg)
    }
}
