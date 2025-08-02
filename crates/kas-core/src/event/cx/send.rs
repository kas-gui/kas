// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context: event send / replay

use super::{EventCx, EventState};
use crate::event::{Command, Event, Scroll, ScrollDelta, Used};
use crate::messages::Erased;
#[allow(unused)] use crate::{Events, Layout};
use crate::{Id, Node};
use std::fmt::Debug;
use std::task::Poll;

impl EventState {
    /// Send a message to `id`
    ///
    /// When calling this method, be aware that some widgets use an inner
    /// component to handle events, thus calling with the outer widget's `id`
    /// may not have the desired effect. [`Layout::try_probe`] and
    /// [`EventState::next_nav_focus`] are usually able to find the appropriate
    /// event-handling target.
    ///
    /// This uses a tree traversal as with event handling, thus ancestors will
    /// have a chance to handle an unhandled event and any messages on the stack
    /// after their child.
    ///
    /// ### Special cases sent as an [`Event`]
    ///
    /// When `M` is `Command`, this will send [`Event::Command`] to the widget.
    ///
    /// When `M` is `ScrollDelta`, this will send [`Event::Scroll`] to the
    /// widget.
    ///
    /// ### Other messages
    ///
    /// The message is pushed to the message stack. The target widget may
    /// [pop](EventCx::try_pop) or [peek](EventCx::try_peek) the message from
    /// [`Events::handle_messages`].
    pub fn send<M: Debug + 'static>(&mut self, id: Id, msg: M) {
        self.send_erased(id, Erased::new(msg));
    }

    /// Push a type-erased message to the stack
    ///
    /// This is a lower-level variant of [`Self::send`].
    pub fn send_erased(&mut self, id: Id, msg: Erased) {
        self.send_queue.push_back((id, msg));
    }

    /// Send a message to `id` via a [`Future`]
    ///
    /// The future is polled after event handling and after drawing and is able
    /// to wake the event loop. This future is executed on the main thread; for
    /// high-CPU tasks use [`Self::send_spawn`] instead.
    ///
    /// The future must resolve to a message on completion. Its message is then
    /// sent to `id` via stack traversal identically to [`Self::send`].
    pub fn send_async<Fut, M>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = M> + 'static,
        M: Debug + 'static,
    {
        self.send_async_erased(id, async { Erased::new(fut.await) });
    }

    /// Send a type-erased message to `id` via a [`Future`]
    ///
    /// This is a low-level variant of [`Self::send_async`].
    pub fn send_async_erased<Fut>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = Erased> + 'static,
    {
        let fut = Box::pin(fut.into_future());
        self.fut_messages.push((id, fut));
    }

    /// Spawn a task, run on a thread pool
    ///
    /// The future is spawned to a thread-pool before the event-handling loop
    /// sleeps, and is able to wake the loop on completion. Tasks involving
    /// significant CPU work should use this method over [`Self::send_async`].
    ///
    /// This method is simply a wrapper around [`async_global_executor::spawn`]
    /// and [`Self::send_async`]; if a different multi-threaded executor is
    /// available, that may be used instead. See also [`async_global_executor`]
    /// documentation of configuration.
    #[cfg(feature = "spawn")]
    pub fn send_spawn<Fut, M>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = M> + 'static,
        Fut::IntoFuture: Send,
        M: Debug + Send + 'static,
    {
        self.send_async(id, async_global_executor::spawn(fut.into_future()));
    }
}

impl<'a> EventCx<'a> {
    /// Get the index of the last child visited
    ///
    /// This is only used when unwinding (traversing back up the widget tree),
    /// and returns the index of the child last visited. E.g. when
    /// [`Events::handle_messages`] is called, this method returns the index of
    /// the child which submitted the message (or whose descendant did).
    /// Otherwise this returns `None` (including when the widget itself is the
    /// submitter of the message).
    pub fn last_child(&self) -> Option<usize> {
        self.last_child
    }

    /// Push a message to the stack
    ///
    /// The message is first type-erased by wrapping with [`Erased`],
    /// then pushed to the stack.
    ///
    /// The message may be [popped](EventCx::try_pop) or
    /// [peeked](EventCx::try_peek) from [`Events::handle_messages`]
    /// by the widget itself, its parent, or any ancestor.
    pub fn push<M: Debug + 'static>(&mut self, msg: M) {
        self.push_erased(Erased::new(msg));
    }

    /// Push a type-erased message to the stack
    ///
    /// This is a lower-level variant of [`Self::push`].
    ///
    /// The message may be [popped](EventCx::try_pop) or
    /// [peeked](EventCx::try_peek) from [`Events::handle_messages`]
    /// by the widget itself, its parent, or any ancestor.
    pub fn push_erased(&mut self, msg: Erased) {
        self.messages.push_erased(msg);
    }

    /// True if the message stack is non-empty
    pub fn has_msg(&self) -> bool {
        self.messages.has_any()
    }

    /// Try popping the last message from the stack with the given type
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_pop<M: Debug + 'static>(&mut self) -> Option<M> {
        self.messages.try_pop()
    }

    /// Try observing the last message on the stack without popping
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_peek<M: Debug + 'static>(&self) -> Option<&M> {
        self.messages.try_peek()
    }

    /// Debug the last message on the stack, if any
    pub fn peek_debug(&self) -> Option<&dyn Debug> {
        self.messages.peek_debug()
    }

    /// Get the message stack operation count
    ///
    /// This is incremented every time the message stack is changed, thus can be
    /// used to test whether a message handler did anything.
    #[inline]
    pub fn msg_op_count(&self) -> usize {
        self.messages.get_op_count()
    }

    /// Set a scroll action
    ///
    /// When setting [`Scroll::Rect`], use the widget's own coordinate space.
    ///
    /// Note that calling this method has no effect on the widget itself, but
    /// affects parents via their [`Events::handle_scroll`] method.
    #[inline]
    pub fn set_scroll(&mut self, scroll: Scroll) {
        self.scroll = scroll;
    }

    pub(crate) fn post_send(&mut self, index: usize) -> Option<Scroll> {
        self.last_child = Some(index);
        (self.scroll != Scroll::None).then_some(self.scroll.clone())
    }

    /// Send a few message types as an Event, replay other messages as if pushed by `id`
    ///
    /// Optionally, push `msg` and set `scroll` as if pushed/set by `id`.
    pub(super) fn send_or_replay(&mut self, mut widget: Node<'_>, id: Id, msg: Erased) {
        if msg.is::<Command>() {
            let cmd = *msg.downcast().unwrap();
            if !self.send_event(widget, id, Event::Command(cmd, None)) {
                match cmd {
                    Command::Exit => self.runner.exit(),
                    Command::Close => self.handle_close(),
                    _ => (),
                }
            }
        } else if msg.is::<ScrollDelta>() {
            let event = Event::Scroll(*msg.downcast().unwrap());
            self.send_event(widget, id, event);
        } else {
            debug_assert!(self.scroll == Scroll::None);
            debug_assert!(self.last_child.is_none());
            self.messages.set_base();
            log::trace!(target: "kas_core::event", "replay: id={id}: {msg:?}");

            self.target_is_disabled = false;
            self.push_erased(msg);
            widget._replay(self, id);
            self.last_child = None;
            self.scroll = Scroll::None;
        }
    }

    /// Replay a scroll action
    #[cfg(feature = "accesskit")]
    pub(super) fn replay_scroll(&mut self, mut widget: Node<'_>, id: Id, scroll: Scroll) {
        log::trace!(target: "kas_core::event", "replay_scroll: id={id}: {scroll:?}");
        debug_assert!(self.scroll == Scroll::None);
        debug_assert!(self.last_child.is_none());
        self.scroll = scroll;
        self.messages.set_base();

        self.target_is_disabled = false;
        widget._replay(self, id);
        self.last_child = None;
        self.scroll = Scroll::None;
    }

    // Call Widget::_send; returns true when event is used
    pub(super) fn send_event(&mut self, mut widget: Node<'_>, mut id: Id, event: Event) -> bool {
        debug_assert!(self.scroll == Scroll::None);
        debug_assert!(self.last_child.is_none());
        self.messages.set_base();
        log::trace!(target: "kas_core::event", "send_event: id={id}: {event:?}");

        // TODO(opt): we should be able to use binary search here
        let mut disabled = false;
        if !event.pass_when_disabled() {
            for d in &self.disabled {
                if d.is_ancestor_of(&id) {
                    id = d.clone();
                    disabled = true;
                }
            }
            if disabled {
                log::trace!(target: "kas_core::event", "target is disabled; sending to ancestor {id}");
            }
        }
        self.target_is_disabled = disabled;

        let used = widget._send(self, id, event) == Used;

        self.last_child = None;
        self.scroll = Scroll::None;
        used
    }

    pub(super) fn poll_futures(&mut self, mut widget: Node<'_>) {
        let mut i = 0;
        while i < self.state.fut_messages.len() {
            let (_, fut) = &mut self.state.fut_messages[i];
            let mut cx = std::task::Context::from_waker(self.runner.waker());
            match fut.as_mut().poll(&mut cx) {
                Poll::Pending => {
                    i += 1;
                }
                Poll::Ready(msg) => {
                    let (id, _) = self.state.fut_messages.remove(i);

                    // Replay message. This could push another future; if it
                    // does we should poll it immediately to start its work.
                    self.send_or_replay(widget.re(), id, msg);
                }
            }
        }
    }
}
