// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget method implementations

use crate::event::{Event, EventCx, FocusSource, IsUsed, Scroll, Unused, Used};
#[cfg(debug_assertions)] use crate::util::IdentifyWidget;
use crate::{Erased, Events, Layout, NavAdvance, Node, Widget, WidgetId};

/// Generic implementation of [`Widget::_send`]
pub fn _send<W: Events>(
    widget: &mut W,
    cx: &mut EventCx,
    data: &<W as Widget>::Data,
    id: WidgetId,
    disabled: bool,
    event: Event,
) -> IsUsed {
    let mut is_used = Unused;
    let do_handle_event;

    if id == widget.id_ref() {
        if disabled {
            return is_used;
        }

        match &event {
            Event::MouseHover(state) => {
                is_used |= widget.handle_hover(cx, *state);
            }
            Event::NavFocus(FocusSource::Key) => {
                cx.set_scroll(Scroll::Rect(widget.rect()));
                is_used |= Used;
            }
            _ => (),
        }

        do_handle_event = true;
    } else {
        if event.is_reusable() {
            is_used = widget.steal_event(cx, data, &id, &event);
        }
        if !is_used {
            cx.assert_post_steal_unused();

            if let Some(index) = widget.find_child_index(&id) {
                let translation = widget.translation();
                let mut _found = false;
                widget.as_node(data).for_child(index, |mut node| {
                    is_used = node._send(cx, id.clone(), disabled, event.clone() + translation);
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

        do_handle_event = !is_used && event.is_reusable();
    }

    if do_handle_event {
        is_used = widget.handle_event(cx, data, event);
    }

    if cx.has_msg() {
        widget.handle_messages(cx, data);
    }

    is_used
}

/// Generic implementation of [`Widget::_replay`]
pub fn _replay<W: Events>(
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
pub fn _nav_next<W: Events>(
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
