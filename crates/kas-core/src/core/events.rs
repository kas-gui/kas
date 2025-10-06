// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget and Events traits

#[allow(unused)] use super::Layout;
use super::{Tile, Widget};
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
/// and involves calling the following methods in order:
///
/// 1.  [`Events::configure`]
/// 2.  [`Events::update`]
/// 3.  [`Events::configure_recurse`]
///
/// Note that both `configure` and `update` may be called before child widgets
/// have been configured. This is important to ensure that parent widgets are
/// always updated before their children.
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
/// An update involves calling [`ConfigCx::update`] or [`EventCx::update`],
/// which then ensure that the following methods are called:
///
/// 1.  [`Events::update`]
/// 2.  [`Events::update_recurse`]
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

    /// Configure children
    ///
    /// # Calling
    ///
    /// This method is called as part of [configuration](Self#configuration).
    ///
    /// Invoke by calling [`ConfigCx::configure`] or [`EventCx::configure`].
    ///
    /// # Implementation
    ///
    /// An explicit implementation is required in cases where not all children
    /// should be configured immediately (for example, a stack or paged list may
    /// choose not to configure hidden children until just before they become
    /// visible). To configure children explicitly, generate an [`Id`] by
    /// calling [`Events::make_child_id`] on `self` then pass this `id` to
    /// [`ConfigCx::configure`].
    ///
    /// The default implementation configures children in the range
    /// [`Tile::child_indices`]. In cases where [`Tile::child_indices`] hides
    /// some children, a custom implementation of this method might be needed.
    fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        for index in self.child_indices().into_iter() {
            let id = self.make_child_id(index);
            if id.is_valid()
                && let Some(node) = self.as_node(data).get_child(index)
            {
                cx.configure(node, id);
            }
        }
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

    /// Update children
    ///
    /// # Calling
    ///
    /// This method is called after [`Self::update`] except during
    /// [configuration](Self#configuration). Children should be updated even if
    /// their data is `()` or is unchanged.
    ///
    /// Invoke by calling [`ConfigCx::update`] or [`EventCx::update`].
    ///
    /// # Implementation
    ///
    /// The default implementation updates children in the range
    /// [`Tile::child_indices`]. This is usually sufficient.
    ///
    /// Use [`ConfigCx::update`].
    fn update_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        for index in self.child_indices().into_iter() {
            if let Some(node) = self.as_node(data).get_child(index) {
                cx.update(node);
            }
        }
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
            cx.redraw(&self);
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
    /// Note that [`Scroll::Rect`] values are in the child's coordinate space,
    /// and must be translated to the widget's own coordinate space by this
    /// method (this is not done by the default implementation since any widget
    /// with non-zero translation very likely wants to implement this method
    /// anyway).
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
