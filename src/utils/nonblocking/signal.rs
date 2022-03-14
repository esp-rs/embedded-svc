/// This module is an adaptation of Embassy's signal (https://github.com/embassy-rs/embassy/blob/master/embassy/src/channel/signal.rs)
/// with a generified Mutex + alloc::sync::Arc where Embassy originally utilizes a critical section.
use core::future::Future;
use core::mem;
use core::task::{Context, Poll, Waker};

extern crate alloc;
use alloc::sync::Arc;

use crate::mutex::Mutex;

/// Synchronization primitive. Allows creating awaitable signals that may be passed between tasks.
/// For a simple use-case where the receiver is only ever interested in the latest value of
/// something, Signals work well.
pub struct Signal<M, T>
where
    M: Mutex<Data = State<T>>,
{
    state: Arc<M>,
}

impl<M, T> Clone for Signal<M, T>
where
    M: Mutex<Data = State<T>>,
{
    fn clone(&self) -> Self {
        Signal {
            state: self.state.clone(),
        }
    }
}

pub enum State<T> {
    None,
    Waiting(Waker),
    Signaled(T),
}

impl<M, T> Signal<M, T>
where
    M: Mutex<Data = State<T>>,
{
    pub fn new() -> Self {
        Self {
            state: Arc::new(M::new(State::None)),
        }
    }
}

impl<M, T> Default for Signal<M, T>
where
    M: Mutex<Data = State<T>>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<M, T: Send> Signal<M, T>
where
    M: Mutex<Data = State<T>>,
{
    /// Mark this Signal as completed.
    pub fn signal(&self, val: T) {
        let mut state = self.state.lock();

        if let State::Waiting(waker) = mem::replace(&mut *state, State::Signaled(val)) {
            waker.wake();
        }
    }

    pub fn reset(&self) {
        let mut state = self.state.lock();

        *state = State::None
    }

    pub fn poll_wait(&self, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.state.lock();

        match &mut *state {
            State::None => {
                *state = State::Waiting(cx.waker().clone());
                Poll::Pending
            }
            State::Waiting(w) if w.will_wake(cx.waker()) => Poll::Pending,
            State::Waiting(_) => panic!("waker overflow"),
            State::Signaled(_) => match mem::replace(&mut *state, State::None) {
                State::Signaled(res) => Poll::Ready(res),
                _ => unreachable!(),
            },
        }
    }

    /// Future that completes when this Signal has been signaled.
    pub fn wait(&self) -> impl Future<Output = T> + '_ {
        futures::future::poll_fn(move |cx| self.poll_wait(cx))
    }

    /// non-blocking method to check whether this signal has been signaled.
    pub fn signaled(&self) -> bool {
        let state = self.state.lock();

        matches!(&*state, State::Signaled(_))
    }
}
