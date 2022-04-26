/// This module is an adaptation of Embassy's signal (https://github.com/embassy-rs/embassy/blob/master/embassy/src/channel/signal.rs)
/// with a generified Mutex where Embassy originally utilizes a critical section.
use core::mem;
use core::task::{Context, Poll, Waker};

use futures::task::AtomicWaker;

use crate::mutex::Mutex;
use crate::signal::asyncs::Signal;
use crate::utils::atomic_swap::AtomicSwap;

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

    fn try_get(&self) -> Option<Self::Data> {
        let data = self.data.swap(None);
        self.waker.take();

        data
    }
}

pub mod adapt {
    use core::convert::Infallible;
    use core::future::Future;

    extern crate alloc;
    use alloc::sync::Arc;

    use crate::channel::asyncs::{Receiver, Sender};
    use crate::errors::Errors;
    use crate::signal::asyncs::Signal;

    struct SignalSender<S, T>(Arc<S>)
    where
        S: Signal<Data = T>;

    impl<S, T> SignalSender<S, T>
    where
        S: Signal<Data = T>,
    {
        pub fn new(signal: Arc<S>) -> Self {
            Self(signal)
        }
    }

    impl<S, T> Clone for SignalSender<S, T>
    where
        S: Signal<Data = T>,
    {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<S, T> Errors for SignalSender<S, T>
    where
        S: Signal<Data = T>,
    {
        type Error = Infallible;
    }

    impl<S, T> Sender for SignalSender<S, T>
    where
        S: Signal<Data = T> + Send + Sync,
        T: Send,
    {
        type Data = T;

        type SendFuture<'a>
        where
            T: 'a,
            S: 'a,
        = impl Future<Output = Result<(), Self::Error>> + Send;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
            let signal = self.0.clone();

            async move {
                signal.signal(value);

                Ok(())
            }
        }
    }

    struct SignalReceiver<S, T>(Arc<S>)
    where
        S: Signal<Data = T>;

    impl<S, T> SignalReceiver<S, T>
    where
        S: Signal<Data = T>,
    {
        pub fn new(signal: Arc<S>) -> Self {
            Self(signal)
        }
    }

    impl<S, T> Clone for SignalReceiver<S, T>
    where
        S: Signal<Data = T>,
    {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<S, T> Errors for SignalReceiver<S, T>
    where
        S: Signal<Data = T>,
    {
        type Error = Infallible;
    }

    impl<S, T> Receiver for SignalReceiver<S, T>
    where
        S: Signal<Data = T> + Send + Sync,
        T: Send,
    {
        type Data = T;

        type RecvFuture<'a>
        where
            T: 'a,
            S: 'a,
        = impl Future<Output = Result<T, Self::Error>> + Send;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            async move {
                let value = futures::future::poll_fn(move |cx| self.0.poll_wait(cx)).await;

                Ok(value)
            }
        }
    }

    pub fn into_sender<S, T>(signal: Arc<S>) -> impl Sender<Data = T>
    where
        S: Signal<Data = T> + Send + Sync,
        T: Send,
    {
        SignalSender::new(signal)
    }

    pub fn into_receiver<S, T>(signal: Arc<S>) -> impl Receiver<Data = T>
    where
        S: Signal<Data = T> + Send + Sync,
        T: Send,
    {
        SignalReceiver::new(signal)
    }
}

#[cfg(feature = "std")]
pub struct MutexSignalFamily;

#[cfg(feature = "std")]
impl crate::signal::asyncs::SignalFamily for MutexSignalFamily {
    type Signal<T> = MutexSignal<std::sync::Mutex<State<T>>, T>;
}

#[cfg(feature = "std")]
impl crate::signal::asyncs::SendSyncSignalFamily for MutexSignalFamily {
    type Signal<T>
    where
        T: Send,
    = MutexSignal<std::sync::Mutex<State<T>>, T>;
}
