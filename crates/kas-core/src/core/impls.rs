// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget method implementations

use crate::event::{ConfigCx, Event, EventCx, FocusSource, Response, Scroll};
#[cfg(debug_assertions)] use crate::util::IdentifyWidget;
use crate::{Erased, Events, Layout, NavAdvance, Node, Widget, WidgetId};

/// Generic implementation of [`Widget::_configure`]
pub fn _configure<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    cx: &mut ConfigCx,
    data: &<W as Widget>::Data,
    id: WidgetId,
) {
    widget.pre_configure(cx, id);

    for index in 0..widget.num_children() {
        let id = widget.make_child_id(index);
        if id.is_valid() {
            widget
                .as_node(data)
                .for_child(index, |mut node| node._configure(cx, id));
        }
    }

    widget.configure(cx);
    widget.update(cx, data);
}

/// Generic implementation of [`Widget::_update`]
pub fn _update<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    cx: &mut ConfigCx,
    data: &<W as Widget>::Data,
) {
    widget.update(cx, data);
    let start = cx.recurse_start.take().unwrap_or(0);
    let end = cx.recurse_end.take().unwrap_or(widget.num_children());
    let mut node = widget.as_node(data);
    for index in start..end {
        node.for_child(index, |mut node| node._update(cx));
    }
}

/// Generic implementation of [`Widget::_send`]
pub fn _send<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    cx: &mut EventCx,
    data: &<W as Widget>::Data,
    id: WidgetId,
    disabled: bool,
    event: Event,
) -> Response {
    let mut response = Response::Unused;
    let do_handle_event;

    if id == widget.id_ref() {
        if disabled {
            return response;
        }

        match &event {
            Event::MouseHover(state) => {
                response |= widget.mouse_hover(cx, *state);
            }
            Event::NavFocus(FocusSource::Key) => {
                cx.set_scroll(Scroll::Rect(widget.rect()));
                response |= Response::Used;
            }
            _ => (),
        }

        do_handle_event = true;
    } else {
        response = widget.steal_event(cx, data, &id, &event);
        if response.is_unused() {
            cx.assert_post_steal_unused();

            if let Some(index) = widget.find_child_index(&id) {
                let translation = widget.translation();
                let mut _found = false;
                widget.as_node(data).for_child(index, |mut node| {
                    response = node._send(cx, id.clone(), disabled, event.clone() + translation);
                    _found = true;
                });

                #[cfg(debug_assertions)]
                if !_found {
                    // This is an error in the widget. It's unlikely and not fatal
                    // so we ignore in release builds.
                    log::error!(
                        "_send: {} found index {index} for {id} but not child",
                        IdentifyWidget(widget.widget_name(), widget.id_ref())
                    );
                }

                if let Some(scroll) = cx.post_send(index) {
                    widget.handle_scroll(cx, data, scroll);
                }
            }
        }

        do_handle_event = response.is_unused() && event.is_reusable();
    }

    if do_handle_event {
        response = widget.handle_event(cx, data, event);
    }

    if cx.has_msg() {
        widget.handle_messages(cx, data);
    }

    response
}

/// Generic implementation of [`Widget::_replay`]
pub fn _replay<W: Widget + Events<Data = <W as Widget>::Data>>(
    widget: &mut W,
    cx: &mut EventCx,
    data: &<W as Widget>::Data,
    id: WidgetId,
    msg: Erased,
) {
    if let Some(index) = widget.find_child_index(&id) {
        let mut _found = false;
        widget.as_node(data).for_child(index, |mut node| {
            node._replay(cx, id.clone(), msg);
            _found = true;
        });

        #[cfg(debug_assertions)]
        if !_found {
            // This is an error in the widget. It's unlikely and not fatal
            // so we ignore in release builds.
            log::error!(
                "_replay: {} found index {index} for {id} but not child",
                IdentifyWidget(widget.widget_name(), widget.id_ref())
            );
        }

        if let Some(scroll) = cx.post_send(index) {
            widget.handle_scroll(cx, data, scroll);
        }

        if cx.has_msg() {
            widget.handle_messages(cx, data);
        }
    } else if id == widget.id_ref() {
        cx.push_erased(msg);
        widget.handle_messages(cx, data);
    } else {
        // This implies use of push_async / push_spawn from a widget which was
        // unmapped or removed.
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
    cx: &mut EventCx,
    data: &<W as Widget>::Data,
    focus: Option<&WidgetId>,
    advance: NavAdvance,
) -> Option<WidgetId> {
    let navigable = widget.navigable();
    nav_next(widget.as_node(data), cx, focus, advance, navigable)
}

fn nav_next(
    mut widget: Node<'_>,
    cx: &mut EventCx,
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
