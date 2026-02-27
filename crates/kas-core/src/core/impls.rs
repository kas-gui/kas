// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget method implementations

use crate::event::{
    ConfigCx, Event, EventCx, EventState, FocusSource, IsUsed, NavAdvance, Scroll, Unused,
};
use crate::{Events, Id, Tile, Widget};

/// Generic implementation of [`Widget::_configure`]
///
/// [`Id`] assignment and and status update are excluded.
#[inline(always)]
pub fn _configure<W: Events>(widget: &mut W, cx: &mut ConfigCx, data: &W::Data) {
    widget.configure(cx);
    widget.update(cx, data);
    for index in widget.recurse_indices().into_iter() {
        let id = widget.make_child_id(index);
        if id.is_valid()
            && let Some(node) = widget.as_node(data).get_child(index)
        {
            cx.configure(node, id);

            // NOTE: we don't handle cx.resize here: resizes are not immediate during configuration
        }
    }
    widget.post_configure(cx);
}

/// Generic implementation of [`Widget::_update`]
///
/// Status update is excluded.
#[inline(always)]
pub fn _update<W: Events>(widget: &mut W, cx: &mut ConfigCx, data: &W::Data) {
    widget.update(cx, data);
    for index in widget.recurse_indices().into_iter() {
        if let Some(node) = widget.as_node(data).get_child(index) {
            cx.update(node);
        }

        if cx.needs_resize() && widget.status().is_sized() {
            let opt_resize = widget.handle_resize(cx, data);
            cx.set_resize(opt_resize);
        }
    }
}

/// Generic implementation of [`Widget::_send`]
#[inline(always)]
pub fn _send<W: Events>(
    widget: &mut W,
    cx: &mut EventCx,
    data: &<W as Widget>::Data,
    id: Id,
    event: Event,
) -> IsUsed {
    let mut is_used = Unused;
    let do_handle_event;

    if id == widget.id_ref() {
        if cx.target_is_disabled {
            return is_used;
        }

        // Side-effects of receiving events at the target widget.
        // These actions do not affect is_used or event propagation.
        match &event {
            Event::MouseOver(state) => {
                widget.handle_mouse_over(cx, *state);
            }
            Event::NavFocus(FocusSource::Key) => {
                cx.set_scroll(Scroll::Rect(widget.rect()));
            }
            _ => (),
        }

        do_handle_event = true;
    } else {
        if let Some(index) = widget.find_child_index(&id) {
            let translation = widget.translation(index);
            let mut _found = false;
            if let Some(mut node) = widget.as_node(data).get_child(index) {
                is_used = node._send(cx, id.clone(), event.clone() + translation);
                _found = true;
            }

            #[cfg(debug_assertions)]
            if !_found {
                // This is an error in the widget. It's unlikely and not fatal
                // so we ignore in release builds.
                log::error!(
                    "_send: {} found index {index} for {id} but not child",
                    widget.identify()
                );
            }

            if cx.needs_resize() {
                debug_assert!(widget.status().is_sized());
                let opt_resize = widget.handle_resize(cx, data);
                cx.set_resize(opt_resize);
            }

            if let Some(scroll) = cx.post_send(index) {
                widget.handle_scroll(cx, data, scroll);
            }

            if matches!(&event, Event::NavFocus(_)) {
                widget.child_nav_focus(cx, id);
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
#[inline(always)]
pub fn _replay<W: Events>(widget: &mut W, cx: &mut EventCx, data: &<W as Widget>::Data, id: Id) {
    if let Some(index) = widget.find_child_index(&id) {
        let mut _found = false;
        if let Some(mut node) = widget.as_node(data).get_child(index) {
            node._replay(cx, id.clone());
            _found = true;
        }

        #[cfg(debug_assertions)]
        if !_found {
            // This is an error in the widget. It's unlikely and not fatal
            // so we ignore in release builds.
            log::error!(
                "_replay: {} found index {index} for {id} but not child",
                widget.identify()
            );
        }

        if cx.needs_resize() && widget.status().is_sized() {
            let opt_resize = widget.handle_resize(cx, data);
            cx.set_resize(opt_resize);
        }

        if let Some(scroll) = cx.post_send(index) {
            widget.handle_scroll(cx, data, scroll);
        }
    } else if cfg!(debug_assertions) && id != widget.id_ref() {
        log::debug!("_replay: broken path {id} after {}", widget.identify());
    }

    if cx.has_msg() {
        widget.handle_messages(cx, data);
    }
}

/// Generic implementation of [`Tile::_nav_next`]
#[inline(always)]
pub fn _nav_next<W: Events>(
    widget: &W,
    cx: &EventState,
    focus: Option<&Id>,
    advance: NavAdvance,
) -> Option<Id> {
    if !widget.navigable() {
        nav_next_non_nav(widget.as_tile(), cx, focus, advance)
    } else {
        nav_next_nav(widget.as_tile(), cx, focus, advance)
    }
}

// Monomorphize nav_next here, not in _nav_next (which would push monomorphization up to the caller)
fn nav_next_non_nav(
    tile: &dyn Tile,
    cx: &EventState,
    focus: Option<&Id>,
    advance: NavAdvance,
) -> Option<Id> {
    nav_next::<false>(tile, cx, focus, advance)
}

fn nav_next_nav(
    tile: &dyn Tile,
    cx: &EventState,
    focus: Option<&Id>,
    advance: NavAdvance,
) -> Option<Id> {
    nav_next::<true>(tile, cx, focus, advance)
}

fn nav_next<const NAVIGABLE: bool>(
    tile: &dyn Tile,
    cx: &EventState,
    focus: Option<&Id>,
    advance: NavAdvance,
) -> Option<Id> {
    let id = tile.id_ref();
    if !id.is_valid() {
        log::warn!("nav_next: encountered unconfigured node!");
        return None;
    } else if cx.is_disabled(id) {
        return None;
    }
    let is_not_focus = *id != focus;

    let mut child = focus.and_then(|id| tile.find_child_index(id));

    if let Some(index) = child {
        let mut opt_id = None;
        let out = &mut opt_id;
        if let Some(node) = tile.get_child(index) {
            *out = node._nav_next(cx, focus, advance);
        }
        if let Some(id) = opt_id {
            return Some(id);
        }
    }

    if NAVIGABLE {
        let can_match_self = match advance {
            NavAdvance::None => true,
            NavAdvance::Forward(true) => true,
            NavAdvance::Forward(false) => is_not_focus,
            _ => false,
        };
        if can_match_self {
            return Some(tile.id_ref().clone());
        }
    }

    let rev = match advance {
        NavAdvance::None => return None,
        NavAdvance::Forward(_) => false,
        NavAdvance::Reverse(_) => true,
    };

    while let Some(index) = tile.nav_next(rev, child) {
        let mut opt_id = None;
        let out = &mut opt_id;
        if let Some(node) = tile.get_child(index) {
            *out = node._nav_next(cx, focus, advance);
        }
        if let Some(id) = opt_id {
            return Some(id);
        }

        child = Some(index);
    }

    if NAVIGABLE {
        let can_match_self = match advance {
            NavAdvance::Reverse(true) => true,
            NavAdvance::Reverse(false) => is_not_focus,
            _ => false,
        };
        if can_match_self {
            return Some(tile.id_ref().clone());
        }
    }

    None
}
