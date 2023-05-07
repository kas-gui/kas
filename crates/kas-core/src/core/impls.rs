// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget method implementations

use crate::event::{ConfigMgr, Event, EventMgr, Response};
use crate::util::IdentifyWidget;
use crate::{Erased, Events, Layout, NavAdvance, NodeMut, Widget, WidgetId};

/// Generic implementation of [`Widget::_configure`]
pub fn _configure<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    data: &<W as Widget>::Data,
    cx: &mut ConfigMgr,
    id: WidgetId,
) {
    widget.pre_configure(cx, id);

    for index in 0..widget.num_children() {
        let id = widget.make_child_id(index);
        if id.is_valid() {
            widget
                .as_node_mut(data)
                .for_child(index, |mut node| node._configure(cx, id));
        }
    }

    widget.configure(data, cx);
    widget.update(data, cx);
}

/// Generic implementation of [`Widget::_update`]
pub fn _update<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    data: &<W as Widget>::Data,
    cx: &mut ConfigMgr,
) {
    widget.update(data, cx);
    if cx.recurse {
        widget
            .as_node_mut(data)
            .for_children(|mut node| node._update(cx));
    }
}

/// Generic implementation of [`Widget::_broadcast`]
pub fn _broadcast<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    data: &<W as Widget>::Data,
    cx: &mut EventMgr,
    count: &mut usize,
    event: Event,
) {
    widget.handle_event(data, cx, event.clone());
    *count += 1;
    widget
        .as_node_mut(data)
        .for_children(|mut node| node._broadcast(cx, count, event.clone()));
}

/// Generic implementation of [`Widget::_send`]
pub fn _send<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    data: &<W as Widget>::Data,
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

        response |= widget.pre_handle_event(data, cx, event);
    } else if widget.steal_event(data, cx, &id, &event).is_used() {
        response = Response::Used;
    } else {
        cx.assert_post_steal_unused();
        if let Some(index) = widget.find_child_index(&id) {
            let translation = widget.translation();
            let mut found = false;
            widget.as_node_mut(data).for_child(index, |mut node| {
                response = node._send(cx, id.clone(), disabled, event.clone() + translation);
                found = true;
            });

            if found {
                if let Some(scroll) = cx.post_send(index) {
                    widget.handle_scroll(data, cx, scroll);
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
            response = widget.handle_event(data, cx, event);
        }
    }

    if cx.has_msg() {
        widget.handle_message(data, cx);
    }

    response
}

/// Generic implementation of [`Widget::_replay`]
pub fn _replay<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    data: &<W as Widget>::Data,
    cx: &mut EventMgr,
    id: WidgetId,
    msg: Erased,
) {
    if let Some(index) = widget.find_child_index(&id) {
        let mut found = false;
        widget.as_node_mut(data).for_child(index, |mut node| {
            node._replay(cx, id.clone(), msg);
            found = true;
        });

        if found {
            if let Some(scroll) = cx.post_send(index) {
                widget.handle_scroll(data, cx, scroll);
            }

            if cx.has_msg() {
                widget.handle_message(data, cx);
            }
        } else {
            #[cfg(debug_assertions)]
            log::warn!(
                "_replay: {} found index {index} for {id} but not child",
                IdentifyWidget(widget.widget_name(), widget.id_ref())
            );
        }
    } else if id == widget.id_ref() {
        cx.push_erased(msg);
        widget.handle_message(data, cx);
    } else {
        #[cfg(debug_assertions)]
        log::debug!(
            "_replay: {} cannot find path to {id}",
            IdentifyWidget(widget.widget_name(), widget.id_ref())
        );
    }
}

/// Generic implementation of [`Widget::_nav_next`]
pub fn _nav_next<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    data: &<W as Widget>::Data,
    cx: &mut EventMgr,
    focus: Option<&WidgetId>,
    advance: NavAdvance,
) -> Option<WidgetId> {
    let navigable = widget.navigable();
    nav_next(widget.as_node_mut(data), cx, focus, advance, navigable)
}

fn nav_next(
    mut widget: NodeMut<'_>,
    cx: &mut EventMgr,
    focus: Option<&WidgetId>,
    advance: NavAdvance,
    navigable: bool,
) -> Option<WidgetId> {
    if cx.is_disabled(widget.id_ref()) {
        return None;
    }

    let mut child = focus.and_then(|id| widget.find_child_index(id));

    if let Some(index) = child {
        if let Some(Some(id)) =
            widget.for_child(index, |mut node| node._nav_next(cx, focus, advance))
        {
            return Some(id);
        }
    }

    if navigable {
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
        if let Some(Some(id)) =
            widget.for_child(index, |mut node| node._nav_next(cx, focus, advance))
        {
            return Some(id);
        }
        child = Some(index);
    }

    if navigable {
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
