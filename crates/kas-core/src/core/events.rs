// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget and Events traits

#[allow(unused)] use super::Layout;
use super::{Tile, Widget};
use crate::ChildIndices;
#[allow(unused)] use crate::event::EventState;
use crate::event::{ConfigCx, CursorIcon, Event, EventCx, IsUsed, Scroll, Unused};
use crate::{Id, geom::Coord};
#[allow(unused)] use kas_macros as macros;

/// Widget event-handling
///
/// This trait governs event handling as part of a [`Widget`] implementation.
/// It is used by the [`#widget`] macro to generate hidden [`Widget`] methods.
///
/// # Implementation
///
/// The implementation of this trait may be omitted where no event-handling is
/// required. All methods have a default implementation.
///
/// ## Foreign items
///
/// The [`#widget`] macro permits implementation of the following items within
/// `impl Events`:
///
/// -   `type` [`Widget::Data`]
///
/// ## Call order
///
/// ### Configuration
///
/// It is required that widgets are configured before other methods are called.
/// This is invoked by calling [`ConfigCx::configure`] or [`EventCx::configure`]
/// and involves the following operations:
///
/// 1.  Set the widget [`Id`], as returned by [`Tile::id`]
/// 2.  Call [`Events::configure`]
/// 3.  Call [`Events::update`]
/// 4.  Recurse configuration to children (see [`Events::recurse_indices`])
/// 5.  Call [`Events::post_configure`]
///
/// Note that both [`configure`](Self::configure) and [`update`](Self::update)
/// may be called before child widgets have been configured. This is important
/// to ensure that parent widgets are always updated before their children. Any
/// logic using child identifiers should be placed in
/// [`post_configure`](Self::post_configure).
///
/// Configuration may be repeated at any time.
///
/// ### Update
///
/// Widgets must be updated during configure (see above), since
/// [`Events::update`] must be called before sizing and before other widget
/// methods.
///
/// Widgets must also be updated after their input data (see [`Widget::Data`])
/// changes (unless not visible, in which case the update may be postponed until
/// they become visible). Updates may happen at other times (mostly because
/// data-change-detection has false positives). Note that custom widgets with
/// state must explicitly update affected children when their state changes.
///
/// An update is invoked by calling [`ConfigCx::update`] or [`EventCx::update`],
/// resulting in the following operations:
///
/// 1.  Call [`Events::update`]
/// 2.  Recurse the update to children (see [`Events::recurse_indices`])
///
/// ### Sizing
///
/// See [`Layout#sizing`].
///
/// It is expected that widgets are sized after [configuration](#configuration)
/// before any other `Events` methods are called, though it is not required that
/// sizing is repeated after re-configuration. It might theoretically be
/// possible to receive a message through [`Events::handle_messages`] before the
/// widget is sized through [`EventState::send`] (or other `send_*` method)
/// being called during configuration or update of this widget or a parent.
///
/// [`#widget`]: macros::widget
pub trait Events: Widget + Sized {
    /// Does this widget have a different appearance on mouse over?
    ///
    /// If `true`, then the mouse moving over and leaving the widget will cause
    /// a redraw. (Note that [`Layout::draw`] can infer the mouse-over state and
    /// start animations.)
    const REDRAW_ON_MOUSE_OVER: bool = false;

    /// The mouse cursor icon to use on mouse over
    ///
    /// Defaults to `None`.
    #[inline]
    fn mouse_over_icon(&self) -> Option<CursorIcon> {
        None
    }

    /// Make an identifier for a child
    ///
    /// This is used to assign children identifiers. It may return
    /// [`Id::default`] in order to avoid configuring the child, but in
    /// this case the widget must configure via another means.
    ///
    /// If this is implemented explicitly then [`Tile::find_child_index`] must
    /// be too.
    ///
    /// Default impl: `self.id_ref().make_child(index)`
    #[inline]
    fn make_child_id(&mut self, index: usize) -> Id {
        self.id_ref().make_child(index)
    }

    /// Probe a coordinate for a widget's [`Id`]
    ///
    /// Returns the [`Id`] of the widget expected to handle clicks and touch
    /// events at the given `coord`. Typically this is the lowest descendant in
    /// the widget tree at the given `coord`, but it is not required to be; e.g.
    /// a `Button` may use an inner widget as a label but return its own [`Id`]
    /// to indicate that the button (not the inner label) handles clicks.
    ///
    /// # Calling
    ///
    /// Call [`Tile::try_probe`] instead.
    ///
    /// # Implementation
    ///
    /// The callee may usually assume that it occupies `coord` and may thus
    /// return its own [`Id`] when no child occupies the input `coord`.
    /// If there are cases where a click within [`Layout::rect`] should be
    /// considered a miss (non-rectangular hit-testing) then
    /// [`Tile::try_probe`] must be implemented instead.
    ///
    /// If the [`Tile::translation`] is non-zero for any child, then the
    /// coordinate passed to that child must be translated:
    /// `coord + translation`.
    ///
    /// ## Default implementation
    ///
    /// The default implementation returns `self.id()` and may be used for
    /// childless widgets. If the [`layout`](macro@crate::layout) attribute
    /// macro is used or an explicit implementation of [`Tile::try_probe`] is
    /// provided, these are used instead of the default implementation of this
    /// method.
    fn probe(&self, coord: Coord) -> Id {
        let _ = coord;
        self.id()
    }

    /// Configure self
    ///
    /// # Calling
    ///
    /// This method is called as part of [configuration](Self#configuration).
    /// This method is called *before* children are assigned identifiers; see
    /// also [`post_configure`](Self::post_configure).
    ///
    /// Invoke by calling [`ConfigCx::configure`] or [`EventCx::configure`].
    ///
    /// # Implementation
    ///
    /// The window's scale factor (and thus any sizes available through
    /// [`ConfigCx::size_cx`]) may change at run-time; this is common since some
    /// platforms require sizing with scale factor 1 before . Such changes require resizing (calling [`Layout::size_rules`]
    /// again) but do not require reconfiguration.
    ///
    /// The default implementation does nothing.
    fn configure(&mut self, cx: &mut ConfigCx) {
        let _ = cx;
    }

    /// Configure self (post-child-configuration actions)
    ///
    /// # Calling
    ///
    /// This method is called as part of [configuration](Self#configuration).
    ///
    /// The default implementation does nothing.
    fn post_configure(&mut self, cx: &mut ConfigCx) {
        let _ = cx;
    }

    /// Update self using input data
    ///
    /// # Calling
    ///
    /// This method is called as part of [configuration](Self#configuration)
    /// and [update](Self#update).
    ///
    /// Invoke by calling [`ConfigCx::update`] or [`EventCx::update`].
    ///
    /// # Implementation
    ///
    /// The default implementation does nothing.
    fn update(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        let _ = (cx, data);
    }

    /// Recursion control
    ///
    /// [Configuration](Self#configuration) and [update](Self#update) normally
    /// recurse to all children listed by [`Tile::child_indices`]; this
    /// recursion is controlled by this method.
    ///
    /// # Calling
    ///
    /// This method is called after [`Self::update`].
    ///
    /// # Implementation
    ///
    /// The default implementation returns the result of [`Tile::child_indices`].
    #[inline]
    fn recurse_indices(&self) -> ChildIndices {
        self.child_indices()
    }

    /// Mouse focus handler
    ///
    /// Called when mouse moves over or leaves this widget.
    /// (This is a low-level alternative
    /// to [`Self::REDRAW_ON_MOUSE_OVER`] and [`Self::mouse_over_icon`].)
    ///
    /// `state` is true when the mouse is over this widget.
    #[inline]
    fn handle_mouse_over(&mut self, cx: &mut EventCx, state: bool) {
        if Self::REDRAW_ON_MOUSE_OVER {
            cx.redraw();
        }
        if state && let Some(icon) = self.mouse_over_icon() {
            cx.set_mouse_over_icon(icon);
        }
    }

    /// Handle an [`Event`]
    ///
    /// This is the primary event handler (see [documentation](crate::event)).
    ///
    /// This method is called on the primary event target. In this case,
    /// [`EventCx::last_child`] returns `None`.
    ///
    /// This method may also be called on ancestors during unwinding (if the
    /// event remains [unused](Unused) and the event
    /// [is reusable](Event::is_reusable)). In this case,
    /// [`EventCx::last_child`] returns `Some(index)` with the index of the
    /// child being unwound from.
    ///
    /// Default implementation of `handle_event`: do nothing; return
    /// [`Unused`].
    fn handle_event(&mut self, cx: &mut EventCx, data: &Self::Data, event: Event) -> IsUsed {
        let _ = (cx, data, event);
        Unused
    }

    /// Handler for messages from children/descendants
    ///
    /// This is the secondary event handler (see [documentation](crate::event)).
    ///
    /// It is implied that the stack contains at least one message.
    /// Use [`EventCx::try_pop`] and/or [`EventCx::try_peek`].
    ///
    /// [`EventCx::last_child`] may be called to find the message's sender.
    /// This may return [`None`] (if no child was visited, which implies that
    /// the message was sent by `self`).
    ///
    /// The default implementation does nothing.
    #[inline]
    fn handle_messages(&mut self, cx: &mut EventCx, data: &Self::Data) {
        let _ = (cx, data);
    }

    /// Handler for scrolling
    ///
    /// When, during [event handling](crate::event), a widget which is a strict
    /// descendant of `self` (i.e. not `self`) calls [`EventCx::set_scroll`]
    /// with a value other than [`Scroll::None`], this method is called.
    ///
    /// If the child is in an independent coordinate space, then this method
    /// should call `cx.set_scroll(Scroll::None)` to avoid any reactions to
    /// child's scroll requests.
    ///
    /// [`EventCx::last_child`] may be called to find the child responsible,
    /// and should never return [`None`] (when called from this method).
    ///
    /// The default implementation does nothing.
    #[inline]
    fn handle_scroll(&mut self, cx: &mut EventCx, data: &Self::Data, scroll: Scroll) {
        let _ = (cx, data, scroll);
    }
}
