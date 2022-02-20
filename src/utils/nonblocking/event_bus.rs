use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use core::time::Duration;

extern crate alloc;
use alloc::sync::Arc;

use futures::future::{ready, Either, Ready};

use crate::channel::nonblocking::{Receiver, Sender};
use crate::errors::Errors;
use crate::event_bus::nonblocking::{EventBus, PostboxProvider};
use crate::mutex::{Condvar, Mutex};
use crate::unblocker::nonblocking::Unblocker;

pub struct AsyncPostbox<U, P, PB> {
    blocking_postbox: PB,
    _payload_type: PhantomData<fn() -> P>,
    _unblocker: PhantomData<fn() -> U>,
}

impl<U, P, PB> AsyncPostbox<U, P, PB> {
    pub fn new(blocking_postbox: PB) -> Self {
        Self {
            blocking_postbox,
            _payload_type: PhantomData,
            _unblocker: PhantomData,
        }
    }
}

impl<U, P, PB> Clone for AsyncPostbox<U, P, PB>
where
    PB: Clone,
{
    fn clone(&self) -> Self {
        Self {
            blocking_postbox: self.blocking_postbox.clone(),
            _payload_type: PhantomData,
            _unblocker: PhantomData,
        }
    }
}

impl<U, P, PB> Errors for AsyncPostbox<U, P, PB>
where
    PB: Errors,
{
    type Error = PB::Error;
}

impl<U, P, PB> Sender for AsyncPostbox<U, P, PB>
where
    U: Unblocker,
    P: Clone + Send + 'static,
    PB: crate::event_bus::Postbox<P> + Clone + Send + 'static,
    Self::Error: Send + Sync + 'static,
{
    type Data = P;

    type SendFuture<'a>
    where
        Self: 'a,
    = Either<Ready<Result<(), Self::Error>>, U::UnblockFuture<Result<(), Self::Error>>>;

    fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
        if !self
            .blocking_postbox
            .post(&value, Some(Duration::from_secs(0)))
            .ok()
            .unwrap_or(false)
        {
            let value = value.clone();
            let mut blocking_postbox = self.blocking_postbox.clone();

            Either::Right(U::unblock(Box::new(move || {
                blocking_postbox.post(&value, None).map(|_| ())
            })))
        } else {
            Either::Left(ready(Ok(())))
        }
    }
}

impl<U, P, PB> super::AsyncWrapper<U, PB> for AsyncPostbox<U, P, PB> {
    fn new(sync: PB) -> Self {
        AsyncPostbox::new(sync)
    }
}

pub struct SubscriptionState<P, S> {
    subscription: Option<S>,
    value: Option<P>,
    waker: Option<Waker>,
}

#[allow(clippy::type_complexity)]
pub struct AsyncSubscription<CV, P, S, E>(
    Arc<(CV::Mutex<SubscriptionState<P, S>>, CV)>,
    PhantomData<fn() -> E>,
)
where
    CV: Condvar,
    P: Send,
    S: Send;

#[cfg(not(feature = "std"))]
impl<CV, P, S, E> Errors for AsyncSubscription<CV, P, S, E>
where
    CV: Condvar,
    P: Send,
    S: Send,
    E: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static,
{
    type Error = E;
}

#[cfg(feature = "std")]
impl<CV, P, S, E> Errors for AsyncSubscription<CV, P, S, E>
where
    CV: Condvar,
    P: Send,
    S: Send,
    E: std::error::Error + Send + Sync + 'static,
{
    type Error = E;
}

#[cfg(not(feature = "std"))]
impl<CV, P, S, E> Receiver for AsyncSubscription<CV, P, S, E>
where
    CV: Condvar,
    S: Send,
    P: Clone + Send,
    E: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static,
{
    type Data = P;

    type RecvFuture<'a>
    where
        Self: 'a,
    = NextFuture<'a, CV, P, S, E>;

    fn recv(&mut self) -> Self::RecvFuture<'_> {
        NextFuture(self)
    }
}

#[cfg(feature = "std")]
impl<CV, P, S, E> Receiver for AsyncSubscription<CV, P, S, E>
where
    CV: Condvar,
    S: Send,
    P: Clone + Send,
    E: std::error::Error + Send + Sync + 'static,
{
    type Data = P;

    type RecvFuture<'a>
    where
        Self: 'a,
    = NextFuture<'a, CV, P, S, E>;

    fn recv(&mut self) -> Self::RecvFuture<'_> {
        NextFuture(self)
    }
}

pub struct NextFuture<'a, CV, P, S, E>(&'a AsyncSubscription<CV, P, S, E>)
where
    CV: Condvar,
    P: Clone + Send,
    S: Send;

impl<'a, CV, P, S, E> Drop for NextFuture<'a, CV, P, S, E>
where
    CV: Condvar,
    P: Clone + Send,
    S: Send,
{
    fn drop(&mut self) {
        let mut state = self.0 .0 .0.lock();

        state.value = None;
        state.waker = None;
    }
}

impl<'a, CV, P, S, E> Future for NextFuture<'a, CV, P, S, E>
where
    CV: Condvar,
    P: Clone + Send,
    S: Send,
{
    type Output = Result<P, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0 .0 .0.lock();

        if let Some(value) = state.value.as_ref() {
            let value = value.clone();

            state.value = None;

            self.0 .0 .1.notify_all();

            Poll::Ready(Ok(value))
        } else {
            state.waker = Some(cx.waker().clone());

            Poll::Pending
        }
    }
}

pub struct Channel<U, CV, E> {
    blocking_channel: E,
    _unblocker: PhantomData<fn() -> U>,
    _condvar_type: PhantomData<fn() -> CV>,
}

impl<U, CV, E> Channel<U, CV, E> {
    pub fn new(blocking_channel: E) -> Self {
        Self {
            blocking_channel,
            _unblocker: PhantomData,
            _condvar_type: PhantomData,
        }
    }
}

impl<U, CV, E> Clone for Channel<U, CV, E>
where
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            blocking_channel: self.blocking_channel.clone(),
            _unblocker: PhantomData,
            _condvar_type: PhantomData,
        }
    }
}

impl<U, CV, E> super::AsyncWrapper<U, E> for Channel<U, CV, E> {
    fn new(sync: E) -> Self {
        Channel::new(sync)
    }
}

impl<U, CV, E> Errors for Channel<U, CV, E>
where
    E: Errors,
{
    type Error = E::Error;
}

impl<U, CV, P, E> EventBus<P> for Channel<U, CV, E>
where
    CV: Condvar + Send + Sync + 'static,
    CV::Mutex<SubscriptionState<P, E::Subscription>>: Send + Sync + 'static,
    P: Clone + Send,
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
{
    type Subscription = AsyncSubscription<CV, P, E::Subscription, E::Error>;

    fn subscribe(&mut self) -> Result<Self::Subscription, Self::Error> {
        let state = Arc::new((
            CV::Mutex::new(SubscriptionState {
                subscription: None,
                value: None,
                waker: None,
            }),
            CV::new(),
        ));

        let subscription_state = Arc::downgrade(&state);

        let subscription = self.blocking_channel.subscribe(move |payload| {
            if let Some(state) = subscription_state.upgrade() {
                let pair: &(CV::Mutex<_>, CV) = &state;

                let (mut state, condvar) = (pair.0.lock(), &pair.1);

                if let Some(a) = mem::replace(&mut state.waker, None) {
                    Waker::wake(a);
                }

                while state.value.is_some() {
                    state = condvar.wait(state);
                }

                state.value = Some(payload.clone());
            }
        })?;

        state.0.lock().subscription = Some(subscription);

        Ok(AsyncSubscription(state, PhantomData))
    }
}

impl<U, CV, P, E> PostboxProvider<P> for Channel<U, CV, E>
where
    U: Unblocker,
    CV: Condvar + Send + Sync + 'static,
    P: Clone + Send + 'static,
    E::Postbox: Clone + Send + 'static,
    E: crate::event_bus::PostboxProvider<P>,
    Self::Error: Send + Sync + 'static,
{
    type Postbox = AsyncPostbox<U, P, E::Postbox>;

    fn postbox(&mut self) -> Result<Self::Postbox, Self::Error> {
        self.blocking_channel.postbox().map(AsyncPostbox::new)
    }
}
