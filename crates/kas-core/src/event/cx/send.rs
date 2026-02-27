// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context: event send / replay

use super::{EventCx, EventState};
use crate::event::{Command, Event, Scroll, ScrollDelta, Used};
use crate::messages::Erased;
use crate::runner::ReadMessage;
#[allow(unused)]
use crate::{Events, Tile, Widget, event::ConfigCx};
use crate::{Id, Node};
use std::fmt::Debug;
use std::task::Poll;

impl EventState {
    /// Send a message to `id`
    ///
    /// # Resolving the target `id`
    ///
    /// If `id` is [valid] (the usual case), the message is sent to this target
    /// using tree traversal (see below). This may be under same or another
    /// window and may or may not actually resolve to a widget.
    ///
    /// If `id` is *not* [valid] and a
    /// [send target](super::ConfigCx::set_send_target_for) has been assigned
    /// for the message's type `M`, `msg` will be sent to that target instead.
    ///
    /// If `id` is *not* [valid] without a type-defined send target, then only
    /// [`AppData::handle_message`] will be called.
    ///
    /// # Sending the message
    ///
    /// Message sending uses tree traversal: resolve the owning window, the
    /// appropriate child widget, its child and so on until the target `id` is
    /// reached. This target widget will may [pop](EventCx::try_pop) or
    /// [peek](EventCx::try_peek) the message from [`Events::handle_messages`].
    /// In case the widget does not pop the message, each parent will get a
    /// chance to do so in its own [`Events::handle_messages`] method until
    /// eventually [`AppData::handle_message`] will be called if no widget
    /// handles the message.
    ///
    /// This tree traversal is mostly the same as that used by the
    /// [event handling model] except that there is no [`Event`]; instead `msg`
    /// is pushed to the message stack directly.
    ///
    /// Tree traversal may fail to reach the target `id` in a number of cases,
    /// for example if the target widget has been removed, remapped (see
    /// [`kas::view`]) or is inaccessible widget (e.g. [`Stack`] may make
    /// inactive pages inaccessible). In this case [`Events::handle_messages`]
    /// will be called as above from the last reachable widget in `id`'s path,
    /// eventually calling [`AppData::handle_message`] as above.
    ///
    /// ## Special cases sent as an [`Event`]
    ///
    /// Some types of message are instead sent as an [`Event`]:
    ///
    /// -   When `M` is [`Command`], this will sent as [`Event::Command`]
    /// -   When `M` is [`ScrollDelta`], this will sent as [`Event::Scroll`]
    ///
    /// In this case the event may be received by [`Events::handle_event`];
    /// see the [event handling model].
    ///
    /// [`AppData::handle_message`]: crate::runner::AppData::handle_message
    /// [valid]: Id::is_valid
    /// [event handling model]: crate::event#event-handling-model
    /// [`kas::view`]: https://docs.rs/kas/latest/kas/view/index.html
    /// [`Stack`]: https://docs.rs/kas/latest/kas/widgets/struct.Stack.html
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
    /// This is a non-blocking variant of [`Self::send`]: when the future `fut`
    /// completes, its result is sent as a message exactly as if it were sent by
    /// [`Self::send`].
    ///
    /// The future is polled after event handling and after drawing and is able
    /// to wake the event loop. This future is executed on the main thread; for
    /// high-CPU tasks use [`Self::send_spawn`] instead.
    ///
    /// ### Cancellation, ordering and failure
    ///
    /// Futures passed to these methods should only be cancelled if they fail to
    /// complete before the window is closed, and even in this case will be
    /// polled at least once.
    ///
    /// Messages sent via `send_async`,
    /// [`send_async_erased`](Self::send_async_erased) and
    /// [`send_spawn`](Self::send_spawn) may be received in any order.
    ///
    /// Message delivery may fail for widgets not currently visible. This is
    /// dependent on widgets implementation of [`Widget::child_node`] (e.g. in
    /// the case of virtual scrolling, the target may have scrolled out of
    /// range).
    pub fn send_async<Fut, M>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = M> + 'static,
        M: Debug + 'static,
    {
        self.send_async_opt(id, async { Some(fut.await) })
    }

    /// Optionally send a message to `id` via a [`Future`]
    ///
    /// This is a variant of [`Self::send_async`].
    pub fn send_async_opt<Fut, M>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = Option<M>> + 'static,
        M: Debug + 'static,
    {
        self.send_async_erased(id, async { fut.await.map(Erased::new) });
    }

    /// Send a type-erased message to `id` via a [`Future`]
    ///
    /// This is a low-level variant of [`Self::send_async`].
    pub fn send_async_erased<Fut>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = Option<Erased>> + 'static,
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
    /// This method simply uses [`async_global_executor`] to spawn a task which
    /// executes on a global thread pool.
    ///
    /// [`async_global_executor`]: https://docs.rs/async-global-executor/
    #[cfg(feature = "spawn")]
    pub fn send_spawn<Fut, M>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = M> + 'static,
        Fut::IntoFuture: Send,
        M: Debug + Send + 'static,
    {
        self.send_async(id, async_global_executor::spawn(fut.into_future()));
    }

    /// Spawn a task, run on a thread pool
    ///
    /// This is a variant of [`Self::send_spawn`].
    #[cfg(feature = "spawn")]
    pub fn send_spawn_opt<Fut, M>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = Option<M>> + 'static,
        Fut::IntoFuture: Send,
        M: Debug + Send + 'static,
    {
        self.send_async_opt(id, async_global_executor::spawn(fut.into_future()));
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
    ///
    /// If not handled during the widget tree traversal and
    /// [a target is set for `M`](ConfigCx::set_send_target_for) then `msg` is
    /// sent to this target.
    ///
    /// Finally, the message may be handled by [`AppData::handle_message`].
    ///
    /// [`AppData::handle_message`]: crate::runner::AppData::handle_message
    pub fn push<M: Debug + 'static>(&mut self, msg: M) {
        self.push_erased(Erased::new(msg));
    }

    /// Push a type-erased message to the stack
    ///
    /// This is a lower-level variant of [`Self::push`].
    pub fn push_erased(&mut self, msg: Erased) {
        self.runner.message_stack_mut().push_erased(msg);
    }

    /// True if the message stack is non-empty
    pub fn has_msg(&self) -> bool {
        self.runner.message_stack().has_any()
    }

    /// Try popping the last message from the stack with the given type
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_pop<M: Debug + 'static>(&mut self) -> Option<M> {
        self.runner.message_stack_mut().try_pop()
    }

    /// Try observing the last message on the stack without popping
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_peek<M: Debug + 'static>(&self) -> Option<&M> {
        self.runner.message_stack().try_peek()
    }

    /// Debug the last message on the stack, if any
    pub fn peek_debug(&self) -> Option<&dyn Debug> {
        self.runner.message_stack().peek_debug()
    }

    /// Get the message stack operation count
    ///
    /// This is incremented every time the message stack is changed, thus can be
    /// used to test whether a message handler did anything.
    #[inline]
    pub fn msg_op_count(&self) -> usize {
        self.runner.message_stack().get_op_count()
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
    pub(super) fn send_or_replay(&mut self, mut widget: Node<'_>, id: Id, mut msg: Erased) {
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
            self.pre_recursion();
            self.runner.message_stack_mut().set_base();
            log::trace!(target: "kas_core::event", "replay: id={id}: {msg:?}");

            self.target_is_disabled = false;
            msg.set_sent();
            self.push_erased(msg);
            widget._replay(self, id);
            self.post_recursion();
        }
    }

    /// Replay a scroll action
    #[cfg(feature = "accesskit")]
    pub(super) fn replay_scroll(&mut self, mut widget: Node<'_>, id: Id, scroll: Scroll) {
        log::trace!(target: "kas_core::event", "replay_scroll: id={id}: {scroll:?}");
        self.pre_recursion();
        self.scroll = scroll;
        self.runner.message_stack_mut().set_base();

        self.target_is_disabled = false;
        widget._replay(self, id);
        self.post_recursion();
    }

    // Call Widget::_send; returns true when event is used
    pub(super) fn send_event(&mut self, mut widget: Node<'_>, mut id: Id, event: Event) -> bool {
        self.pre_recursion();
        self.runner.message_stack_mut().set_base();
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

        self.post_recursion();
        used
    }

    pub(super) fn poll_futures(&mut self) {
        let mut i = 0;
        while i < self.state.fut_messages.len() {
            let (_, fut) = &mut self.cx.state.fut_messages[i];
            let mut cx = std::task::Context::from_waker(self.runner.waker());
            match fut.as_mut().poll(&mut cx) {
                Poll::Pending => {
                    i += 1;
                }
                Poll::Ready(opt_msg) => {
                    let (id, _) = self.state.fut_messages.remove(i);

                    // Send via queue to support send targets and inter-window sending
                    if let Some(msg) = opt_msg {
                        self.send_queue.push_back((id, msg));
                    }
                }
            }
        }
    }
}
