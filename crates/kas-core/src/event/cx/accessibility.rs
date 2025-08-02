// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — platform API

use super::{EventCx, EventState};
use crate::cast::CastApprox;
use crate::event::{Command, Event, FocusSource, Scroll, ScrollDelta};
use crate::geom::{Rect, Size};
use crate::window::Window;
use crate::{Id, Node, TileExt};

impl EventState {
    /// True if [AccessKit](https://accesskit.dev/) is enabled
    #[inline]
    pub(crate) fn accesskit_is_enabled(&self) -> bool {
        self.accesskit_is_enabled
    }

    pub(crate) fn accesskit_tree_update<A>(&mut self, root: &Window<A>) -> accesskit::TreeUpdate {
        self.accesskit_is_enabled = true;

        let (nodes, root_id) = crate::accesskit::window_nodes(root);
        let tree = Some(accesskit::Tree::new(root_id));

        // AccessKit does not like focus to point at a non-existant node, so we
        // filter. See https://github.com/AccessKit/accesskit/issues/587
        let focus = self
            .nav_focus()
            .map(|id| id.into())
            .filter(|node_id| nodes.iter().any(|(id, _)| id == node_id))
            .unwrap_or(root_id);

        accesskit::TreeUpdate { nodes, tree, focus }
    }

    pub(crate) fn disable_accesskit(&mut self) {
        self.accesskit_is_enabled = false;
    }
}

impl<'a> EventCx<'a> {
    pub(crate) fn handle_accesskit_action(
        &mut self,
        widget: Node<'_>,
        request: accesskit::ActionRequest,
    ) {
        let Some(id) = Id::try_from_u64(request.target.0) else {
            return;
        };

        // TODO: implement remaining actions
        use crate::messages::{self, Erased, SetValueF64, SetValueText};
        use accesskit::{Action as AKA, ActionData};
        match request.action {
            AKA::Click => {
                self.send_event(widget, id, Event::Command(Command::Activate, None));
            }
            AKA::Focus => self.set_nav_focus(id, FocusSource::Synthetic),
            AKA::Blur => (),
            AKA::Collapse | AKA::Expand => (), // TODO: open/close menus
            AKA::CustomAction => (),
            AKA::Decrement => {
                self.send_or_replay(widget, id, Erased::new(messages::DecrementStep));
            }
            AKA::Increment => {
                self.send_or_replay(widget, id, Erased::new(messages::IncrementStep));
            }
            AKA::HideTooltip | AKA::ShowTooltip => (),
            AKA::ReplaceSelectedText => (),
            AKA::ScrollDown | AKA::ScrollLeft | AKA::ScrollRight | AKA::ScrollUp => {
                let delta = match request.action {
                    AKA::ScrollDown => ScrollDelta::Lines(0.0, 1.0),
                    AKA::ScrollLeft => ScrollDelta::Lines(-1.0, 0.0),
                    AKA::ScrollRight => ScrollDelta::Lines(1.0, 0.0),
                    AKA::ScrollUp => ScrollDelta::Lines(0.0, -1.0),
                    _ => unreachable!(),
                };
                self.send_event(widget, id, Event::Scroll(delta));
            }
            AKA::ScrollIntoView | AKA::ScrollToPoint => {
                // We assume input is in coordinate system of target
                let scroll = match request.data {
                    None => {
                        debug_assert_eq!(request.action, AKA::ScrollIntoView);
                        // NOTE: we shouldn't need two tree traversals, but it's fine
                        if let Some(tile) = widget.as_tile().find_tile(&id) {
                            Scroll::Rect(tile.rect())
                        } else {
                            return;
                        }
                    }
                    Some(ActionData::ScrollToPoint(point)) => {
                        debug_assert_eq!(request.action, AKA::ScrollToPoint);
                        let pos = point.cast_approx();
                        let size = Size::ZERO;
                        Scroll::Rect(Rect { pos, size })
                    }
                    _ => {
                        debug_assert!(false);
                        return;
                    }
                };
                self.replay_scroll(widget, id, scroll);
            }
            AKA::SetScrollOffset => {
                if let Some(ActionData::SetScrollOffset(point)) = request.data {
                    let msg = kas::messages::SetScrollOffset(point.cast_approx());
                    self.send_or_replay(widget, id, Erased::new(msg));
                }
            }
            AKA::SetTextSelection => (),
            AKA::SetSequentialFocusNavigationStartingPoint => (),
            AKA::SetValue => {
                let msg = match request.data {
                    Some(ActionData::Value(text)) => Erased::new(SetValueText(text.into())),
                    Some(ActionData::NumericValue(n)) => Erased::new(SetValueF64(n)),
                    _ => return,
                };
                self.send_or_replay(widget, id, msg);
            }
            AKA::ShowContextMenu => (),
        }
    }
}
