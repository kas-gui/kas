// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: Response type

use super::{Manager, VoidResponse};
use crate::geom::{Offset, Rect};
use crate::Widget;

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
    /// Nothing of external interest
    ///
    /// This implies that the event was consumed, but does not affect parents.
    /// Note that we consider "view changes" (i.e. scrolling) to not be of
    /// external interest.
    None,
    /// Unhandled event
    ///
    /// Indicates that the event was not consumed. An ancestor or the event
    /// manager is thus able to make use of this event.
    Unhandled,
    /// Pan scrollable regions by the given delta
    ///
    /// With the usual scroll offset conventions, this delta must be subtracted
    /// from the scroll offset.
    Pan(Offset),
    /// (Keyboard) focus has changed. This region should be made visible.
    Focus(Rect),
    /// Widget wishes to be selected (or have selection status toggled)
    Select,
    /// Notify of update to widget's data
    ///
    /// Widgets which hold editable data should return either this or
    /// [`Response::Msg`] on handling events which update that data.
    /// Note: scrolling/adjusting a view is not considered a data update.
    Update,
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
    pub fn none_or_msg(opt_msg: Option<M>) -> Self {
        match opt_msg {
            None => Response::None,
            Some(msg) => Response::Msg(msg),
        }
    }

    /// Construct `Update` or `Msg(msg)`
    #[inline]
    pub fn update_or_msg(opt_msg: Option<M>) -> Self {
        match opt_msg {
            None => Response::Update,
            Some(msg) => Response::Msg(msg),
        }
    }

    /// True if variant is `None`
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, Response::None)
    }

    /// True if variant is `Unhandled`
    #[inline]
    pub fn is_unhandled(&self) -> bool {
        matches!(self, Response::Unhandled)
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
            None => Ok(None),
            Unhandled => Ok(Unhandled),
            Pan(delta) => Ok(Pan(delta)),
            Focus(rect) => Ok(Focus(rect)),
            Select => Ok(Select),
            Update => Ok(Update),
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

/// Configurable message converter / handler
///
/// Parent widgets are expected to implement this to handle or convert messages
/// from child widgets, excepting where the parent and child message types are
/// equal (which is implemented as pass-through).
pub trait OnMessage<M>: Widget {
    /// Called on a widget:
    ///
    /// -   `mgr`: the event manager
    /// -   `index`: index of child widget
    /// -   `msg`: message from child
    fn on_msg(&mut self, mgr: &mut Manager, index: usize, msg: M) -> Response<Self::Msg>;
}

// TODO: This impl is required, yet falsely reported to cause conflicts
// Bug report: https://github.com/rust-lang/rust/issues/90587
impl<W: Widget, M: Into<W::Msg>> OnMessage<M> for W {
    #[inline]
    fn on_msg(&mut self, _mgr: &mut Manager, _index: usize, msg: M) -> Response<W::Msg> {
        Response::Msg(msg.into())
    }
}

// Below alternatives are not viable: some widgets like CheckBox are generic
// over M and required implementations hit the same conflict.

// impl<W: Widget> OnMessage<VoidMsg> for W {
//     #[inline]
//     fn on_msg(&mut self, _: &mut Manager, _: usize, _: VoidMsg) -> Response<W::Msg> {
//         Response::None
//     }
// }

// impl<W: Widget> OnMessage<W::Msg> for W {
//     #[inline]
//     fn on_msg(&mut self, _: &mut Manager, _: usize, msg: W::Msg) -> Response<W::Msg> {
//         Response::Msg(msg)
//     }
// }
