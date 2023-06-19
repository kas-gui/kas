// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget method implementations

use crate::event::{ConfigMgr, Event, EventMgr, Response};
use crate::util::IdentifyWidget;
use crate::{Erased, Events, Layout, NavAdvance, WidgetId};

/// Generic implementation of [`Widget::_configure`]
pub fn _configure<W: Layout + Events>(widget: &mut W, cx: &mut ConfigMgr, id: WidgetId) {
    widget.pre_configure(cx, id);

    for index in 0..widget.num_children() {
        let id = widget.make_child_id(index);
        if id.is_valid() {
            if let Some(widget) = widget.get_child_mut(index) {
                widget._configure(cx, id);
            }
        }
    }

    widget.configure(cx);
}

/// Generic implementation of [`Widget::_broadcast`]
pub fn _broadcast<W: Layout + Events>(
    widget: &mut W,
    cx: &mut EventMgr,
    count: &mut usize,
    event: Event,
) {
    widget.handle_event(cx, event.clone());
    *count += 1;
    for index in 0..widget.num_children() {
        if let Some(w) = widget.get_child_mut(index) {
            w._broadcast(cx, count, event.clone());
        }
    }
}

/// Generic implementation of [`Widget::_send`]
pub fn _send<W: Layout + Events>(
    widget: &mut W,
    cx: &mut EventMgr,
    id: WidgetId,
    disabled: bool,
    event: Event,
) -> Response {
    let mut response = Response::Unused;
    if id == widget.id_ref() {
        if disabled {
            return response;
        }

        response |= widget.pre_handle_event(cx, event);
    } else if widget.steal_event(cx, &id, &event).is_used() {
        response = Response::Used;
    } else {
        cx.assert_post_steal_unused();
        if let Some(index) = widget.find_child_index(&id) {
            let translation = widget.translation();
            if let Some(w) = widget.get_child_mut(index) {
                response = w._send(cx, id, disabled, event.clone() + translation);
                if let Some(scroll) = cx.post_send(index) {
                    widget.handle_scroll(cx, scroll);
                }
            } else {
                #[cfg(debug_assertions)]
                log::warn!(
                    "_send: {} found index {index} for {id} but not child",
                    IdentifyWidget(widget.widget_name(), widget.id_ref())
                );
            }
        }

        if response.is_unused() && event.is_reusable() {
            response = widget.handle_event(cx, event);
        }
    }

    if cx.has_msg() {
        widget.handle_message(cx);
    }

    response
}

/// Generic implementation of [`Widget::_replay`]
pub fn _replay<W: Layout + Events>(widget: &mut W, cx: &mut EventMgr, id: WidgetId, msg: Erased) {
    if let Some(index) = widget.find_child_index(&id) {
        if let Some(w) = widget.get_child_mut(index) {
            w._replay(cx, id, msg);
            if let Some(scroll) = cx.post_send(index) {
                widget.handle_scroll(cx, scroll);
            }
        } else {
            #[cfg(debug_assertions)]
            log::warn!(
                "_replay: {} found index {index} for {id} but not child",
                IdentifyWidget(widget.widget_name(), widget.id_ref())
            );
        }

        if cx.has_msg() {
            widget.handle_message(cx);
        }
    } else if id == widget.id_ref() {
        cx.push_erased(msg);
        widget.handle_message(cx);
    } else {
        #[cfg(debug_assertions)]
        log::debug!(
            "_replay: {} cannot find path to {id}",
            IdentifyWidget(widget.widget_name(), widget.id_ref())
        );
    }
}

/// Generic implementation of [`Widget::_nav_next`]
pub fn _nav_next<W: Layout + Events>(
    widget: &mut W,
    cx: &mut EventMgr,
    focus: Option<&WidgetId>,
    advance: NavAdvance,
) -> Option<WidgetId> {
    if cx.is_disabled(widget.id_ref()) {
        return None;
    }

    let mut child = focus.and_then(|id| widget.find_child_index(id));

    if let Some(index) = child {
        if let Some(id) = widget
            .get_child_mut(index)
            .and_then(|w| w._nav_next(cx, focus, advance))
        {
            return Some(id);
        }
    }

    if widget.navigable() {
        let can_match_self = match advance {
            NavAdvance::None => true,
            NavAdvance::Forward(true) => true,
            NavAdvance::Forward(false) => *widget.id_ref() != focus,
            _ => false,
        };
        if can_match_self {
            return Some(widget.id_ref().clone());
        }
    }

    let rev = match advance {
        NavAdvance::None => return None,
        NavAdvance::Forward(_) => false,
        NavAdvance::Reverse(_) => true,
    };

    while let Some(index) = widget.nav_next(rev, child) {
        if let Some(id) = widget
            .get_child_mut(index)
            .and_then(|w| w._nav_next(cx, focus, advance))
        {
            return Some(id);
        }
        child = Some(index);
    }

    if widget.navigable() {
        let can_match_self = match advance {
            NavAdvance::Reverse(true) => true,
            NavAdvance::Reverse(false) => *widget.id_ref() != focus,
            _ => false,
        };
        if can_match_self {
            return Some(widget.id_ref().clone());
        }
    }

    None
}
