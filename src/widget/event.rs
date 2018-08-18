//! Event handling
//! 
//! Event handling uses *event* messages, passed from the parent into a widget,
//! with responses passed back to the parent.

/// Input events
pub enum Event {
}

/// Simple variant when no response is delivered
pub enum NoResponse {
    /// Success, no response
    None,
}

/// Pre-defined event repsonses
pub enum Response {
    /// Success, no response
    None,
    /// Close the window
    Close,
}

impl From<NoResponse> for Response {
    fn from(r: NoResponse) -> Self {
        match r {
            NoResponse::None => Response::None
        }
    }
}

pub fn ignore<M: From<NoResponse>>(_: Event) -> M {
    M::from(NoResponse::None)
}
