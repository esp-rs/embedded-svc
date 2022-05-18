/// This module is an adaptation of Embassy's signal (https://github.com/embassy-rs/embassy/blob/master/embassy/src/channel/signal.rs)
/// with a generified Mutex where Embassy originally utilizes a critical section.
use core::mem;
use core::task::{Context, Poll, Waker};

use crate::mutex::Mutex;
use crate::signal::asyncs::Signal;

#[cfg(target_has_atomic = "ptr")]
pub use atomic_signal::*;

/// Synchronization primitive. Allows creating awaitable signals that may be passed between tasks.
/// For a simple use-case where the receiver is only ever interested in the latest value of
/// something, Signals work well.
pub struct MutexSignal<M, T>(M)
where
    M: Mutex<Data = State<T>>;

impl<M, T> Clone for MutexSignal<M, T>
where
    M: Mutex<Data = State<T>> + Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub enum State<T> {
    None,
    Waiting(Waker),
    Signaled(T),
}

impl<M, T> MutexSignal<M, T>
where
    M: Mutex<Data = State<T>>,
{
    pub fn new() -> Self {
        Self(M::new(State::None))
    }

    pub fn signaled(&self) -> bool {
        let state = self.0.lock();

        matches!(&*state, State::Signaled(_))
    }
}

impl<M, T> Default for MutexSignal<M, T>
where
    M: Mutex<Data = State<T>>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<M, T> Signal for MutexSignal<M, T>
where
    M: Mutex<Data = State<T>>,
{
    type Data = T;

    fn new() -> Self {
        Default::default()
    }

    fn reset(&self) {
        let mut state = self.0.lock();

        *state = State::None
    }

    fn signal(&self, data: T) {
        let mut state = self.0.lock();

        if let State::Waiting(waker) = mem::replace(&mut *state, State::Signaled(data)) {
            waker.wake();
        }
    }

    fn poll_wait(&self, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.0.lock();

        match &mut *state {
            State::None => {
                *state = State::Waiting(cx.waker().clone());
                Poll::Pending
            }
            State::Waiting(w) if w.will_wake(cx.waker()) => Poll::Pending,
            State::Waiting(_) => panic!("waker overflow"),
            State::Signaled(_) => match mem::replace(&mut *state, State::None) {
                State::Signaled(data) => Poll::Ready(data),
                _ => unreachable!(),
            },
        }
    }

    fn is_set(&self) -> bool {
        let state = self.0.lock();

        matches!(&*state, State::Signaled(_))
    }

    fn try_get(&self) -> Option<Self::Data> {
        let mut state = self.0.lock();

        match &mut *state {
            State::Signaled(_) => match mem::replace(&mut *state, State::None) {
                State::Signaled(res) => Some(res),
                _ => unreachable!(),
            },
            _ => None,
        }
    }
}

#[cfg(target_has_atomic = "ptr")]
mod atomic_signal {
    use core::task::{Context, Poll};

    use futures::task::AtomicWaker;

    use crate::signal::asyncs::Signal;
    use crate::utils::atomic_swap::AtomicSwap;

    pub struct AtomicSignal<S, T>
    where
        S: AtomicSwap<Data = Option<T>>,
    {
        waker: AtomicWaker,
        data: S,
    }

    impl<S, T> AtomicSignal<S, T>
    where
        S: AtomicSwap<Data = Option<T>>,
    {
        pub fn new() -> Self {
            Self {
                data: S::new(None),
                waker: AtomicWaker::new(),
            }
        }
    }

    impl<S, T> Default for AtomicSignal<S, T>
    where
        S: AtomicSwap<Data = Option<T>>,
    {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<S, T> Signal for AtomicSignal<S, T>
    where
        S: AtomicSwap<Data = Option<T>>,
        T: Copy,
    {
        type Data = T;

        fn new() -> Self {
            Default::default()
        }

        fn reset(&self) {
            self.data.swap(None);
            self.waker.take();
        }

        fn signal(&self, data: T) {
            self.data.swap(Some(data));
            self.waker.wake();
        }

        fn poll_wait(&self, cx: &mut Context<'_>) -> Poll<T> {
            self.waker.register(cx.waker());

            if let Some(data) = self.data.swap(None) {
                Poll::Ready(data)
            } else {
                Poll::Pending
            }
        }

        fn is_set(&self) -> bool {
            self.data.load().is_some()
        }

        fn try_get(&self) -> Option<Self::Data> {
            let data = self.data.swap(None);
            self.waker.take();

            data
        }
    }
}

pub mod adapt {
    use core::convert::Infallible;
    use core::future::Future;

    use crate::channel::asyncs::{Receiver, Sender};
    use crate::errors::Errors;
    use crate::signal::asyncs::Signal;

    pub fn as_channel<S, T>(signal: &'static S) -> SignalChannel<'static, S, T>
    where
        S: Signal<Data = T> + Send + Sync,
        T: Send + 'static,
    {
        SignalChannel::new(signal)
    }

    pub struct SignalChannel<'a, S, T>(&'a S)
    where
        S: Signal<Data = T> + 'a;

    impl<'a, S, T> SignalChannel<'a, S, T>
    where
        S: Signal<Data = T> + 'a,
    {
        pub fn new(signal: &'a S) -> Self {
            Self(signal)
        }
    }

    impl<'a, S, T> Errors for SignalChannel<'a, S, T>
    where
        S: Signal<Data = T> + 'a,
    {
        type Error = Infallible;
    }

    impl<'s, S, T> Sender for SignalChannel<'s, S, T>
    where
        S: Signal<Data = T> + Send + Sync + 's,
        T: Send,
    {
        type Data = T;

        type SendFuture<'a>
        where
            T: 'a,
            S: 'a,
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>> + Send;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
            #[allow(clippy::clone_double_ref)]
            let signal = self.0.clone();

            async move {
                signal.signal(value);

                Ok(())
            }
        }
    }

    impl<'s, S, T> Receiver for SignalChannel<'s, S, T>
    where
        S: Signal<Data = T> + Send + Sync + 's,
        T: Send,
    {
        type Data = T;

        type RecvFuture<'a>
        where
            T: 'a,
            S: 'a,
            Self: 'a,
        = impl Future<Output = Result<T, Self::Error>> + Send;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            async move {
                let value = futures::future::poll_fn(move |cx| self.0.poll_wait(cx)).await;

                Ok(value)
            }
        }
    }
}

#[cfg(feature = "std")]
impl crate::signal::asyncs::SignalFamily for std::sync::Condvar {
    type Signal<T> = MutexSignal<std::sync::Mutex<State<T>>, T>;
}

#[cfg(feature = "std")]
impl crate::signal::asyncs::SendSyncSignalFamily for std::sync::Condvar {
    type Signal<T>
    where
        T: Send,
    = MutexSignal<std::sync::Mutex<State<T>>, T>;
}
