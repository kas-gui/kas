// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget and Events traits

#[allow(unused)] use super::Layout;
use super::{Tile, Widget};
use crate::Id;
#[allow(unused)] use crate::event::EventState;
use crate::event::{ConfigCx, CursorIcon, Event, EventCx, IsUsed, Scroll, Unused};
#[allow(unused)] use kas_macros as macros;

/// Widget event-handling
///
/// This trait governs event handling as part of a [`Widget`] implementation.
/// It is used by the [`#widget`] macro to generate hidden [`Widget`] methods.
///
/// The implementation of this method may be omitted where no event-handling is
/// required. All methods have a default implementation.
///
/// Type [`Widget::Data`] may be specified in `impl Events { ... }` instead of
/// in `impl Widget { ... }` (this is permitted since it allows may `#[widget]`
/// definitions to omit `impl Widget { ... }` altogether).
///
/// # Widget lifecycle
///
/// 1.  The widget is configured ([`Events::configure`],
///     [`Events::configure_recurse`]) and immediately updated
///     ([`Events::update`]).
///
///     The widget may be re-configured at any time without expectation that
///     the layout will be resized / set again.
/// 2.  The widget is updated by calling [`Events::update`] immediately after
///     it is configured and also after any update to input data (or other data
///     which may have changed, such as that exposed by [`EventState::config`]).
/// 3.  The widget is "sized" by calling [`Layout::size_rules`] for the
///     horizontal axis then for the vertical axis.
///
///     These methods may be called again at any time without expectation that
///     the layout will be set again.
/// 4.  [`Layout::set_rect`] is called to "set" layout.
///
///     This method may be called again at any time.
/// 5.  The widget is ready for event-handling and drawing
///     ([`Events::handle_event`], [`Layout::try_probe`], [`Layout::draw`]).
///
/// Widgets are responsible for ensuring that their children may observe this
/// lifecycle. Usually this simply involves inclusion of the child in layout
/// operations. Steps of the lifecycle may be postponed until a widget becomes
/// visible.
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

    /// Configure self
    ///
    /// Widgets are *configured* before sizing, drawing and event handling (see
    /// [widget lifecycle](Widget#widget-lifecycle)).
    ///
    /// Configuration may be repeated at any time. If `id` changes, children
    /// must be assigned new/updated identifiers.
    ///
    /// [`Self::update`] is always called immediately after this method,
    /// followed by [`Self::configure_recurse`].
    ///
    /// The window's scale factor (and thus any sizes available through
    /// [`ConfigCx::size_cx`]) may not be correct initially (some platforms
    /// construct all windows using scale factor 1) and/or may change in the
    /// future. Changes to the scale factor result in recalculation of
    /// [`Layout::size_rules`] but not repeated configuration.
    ///
    /// The default implementation does nothing.
    fn configure(&mut self, cx: &mut ConfigCx) {
        let _ = cx;
    }

    /// Configure children
    ///
    /// This method is called after [`Self::configure`].
    /// The default implementation configures all children.
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
    /// This method is called immediately after [`Self::configure`] and after
    /// any input data is updated, before [`Layout::draw`] is called.
    /// Typically this method is called immediately after the data is updated
    /// but the call may be delayed until when the widget becomes visible.
    ///
    /// This method is called before [`Self::update_recurse`].
    ///
    /// The default implementation does nothing.
    fn update(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        let _ = (cx, data);
    }

    /// Update children
    ///
    /// This method is called after [`Self::update`]. It usually configures all
    /// children. Children should be updated even if their data is `()` or is
    /// unchanged.
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
