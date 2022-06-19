use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

extern crate alloc;
use alloc::sync::Arc;

use crate::channel::asynch::{Receiver, Sender};
use crate::event_bus::asynch::{ErrorType, EventBus, PostboxProvider};
use crate::mutex::{Condvar, Mutex, MutexFamily};
use crate::unblocker::asynch::Unblocker;

pub struct AsyncPostbox<U, P, PB> {
    unblocker: U,
    blocking_postbox: PB,
    _payload_type: PhantomData<fn() -> P>,
}

impl<U, P, PB> AsyncPostbox<U, P, PB> {
    pub fn new(unblocker: U, blocking_postbox: PB) -> Self {
        Self {
            unblocker,
            blocking_postbox,
            _payload_type: PhantomData,
        }
    }
}

impl<U, P, PB> Clone for AsyncPostbox<U, P, PB>
where
    U: Clone,
    PB: Clone,
{
    fn clone(&self) -> Self {
        Self {
            unblocker: self.unblocker.clone(),
            blocking_postbox: self.blocking_postbox.clone(),
            _payload_type: PhantomData,
        }
    }
}

impl<U, P, PB> Sender for AsyncPostbox<U, P, PB>
where
    U: Unblocker,
    P: Clone + Send + 'static,
    PB: crate::event_bus::Postbox<P> + Clone + Send + 'static,
{
    type Data = P;

    type SendFuture<'a>
    where
        Self: 'a,
    = U::UnblockFuture<()>;

    fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
        let value = value;
        let mut blocking_postbox = self.blocking_postbox.clone();

        self.unblocker
            .unblock(move || blocking_postbox.post(&value, None).map(|_| ()).unwrap())
    }
}

impl<P, PB> Sender for AsyncPostbox<(), P, PB>
where
    P: Clone + Send + 'static,
    PB: crate::event_bus::Postbox<P> + Clone + Send + 'static,
{
    type Data = P;

    type SendFuture<'a>
    where
        Self: 'a,
    = impl Future<Output = ()>;

    fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
        async move {
            self.blocking_postbox
                .post(&value, None)
                .map(|_| ())
                .unwrap()
        }
    }
}

impl<P, PB> super::AsyncWrapper<PB> for AsyncPostbox<(), P, PB> {
    fn new(sync: PB) -> Self {
        AsyncPostbox::new((), sync)
    }
}

impl<U, P, PB> super::UnblockingAsyncWrapper<U, PB> for AsyncPostbox<U, P, PB> {
    fn new(unblocker: U, sync: PB) -> Self {
        AsyncPostbox::new(unblocker, sync)
    }
}

pub struct SubscriptionState<P, S> {
    subscription: Option<S>,
    value: Option<P>,
    waker: Option<Waker>,
}

#[allow(clippy::type_complexity)]
pub struct AsyncSubscription<CV, P, S>(Arc<(CV::Mutex<SubscriptionState<P, S>>, CV)>)
where
    CV: Condvar,
    P: Send,
    S: Send;

impl<CV, P, S> Receiver for AsyncSubscription<CV, P, S>
where
    CV: Condvar + Send + Sync,
    <CV as MutexFamily>::Mutex<SubscriptionState<P, S>>: Send + Sync,
    S: Send,
    P: Clone + Send,
{
    type Data = P;

    type RecvFuture<'a>
    where
        Self: 'a,
    = NextFuture<'a, CV, P, S>;

    fn recv(&mut self) -> Self::RecvFuture<'_> {
        NextFuture(self)
    }
}

pub struct NextFuture<'a, CV, P, S>(&'a AsyncSubscription<CV, P, S>)
where
    CV: Condvar + Send + Sync,
    <CV as MutexFamily>::Mutex<SubscriptionState<P, S>>: Send + Sync,
    P: Clone + Send,
    S: Send;

impl<'a, CV, P, S> Drop for NextFuture<'a, CV, P, S>
where
    CV: Condvar + Send + Sync,
    <CV as MutexFamily>::Mutex<SubscriptionState<P, S>>: Send + Sync,
    P: Clone + Send,
    S: Send,
{
    fn drop(&mut self) {
        let mut state = self.0 .0 .0.lock();
        state.waker = None;
    }
}

impl<'a, CV, P, S> Future for NextFuture<'a, CV, P, S>
where
    CV: Condvar + Send + Sync,
    <CV as MutexFamily>::Mutex<SubscriptionState<P, S>>: Send + Sync,
    P: Clone + Send,
    S: Send,
{
    type Output = P;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0 .0 .0.lock();

        let value = mem::replace(&mut state.value, None);

        if let Some(value) = value {
            self.0 .0 .1.notify_all();

            Poll::Ready(value)
        } else {
            state.waker = Some(cx.waker().clone());

            self.0 .0 .1.notify_all();

            Poll::Pending
        }
    }
}

pub struct AsyncEventBus<U, CV, E> {
    unblocker: U,
    event_bus: E,
    _condvar_type: PhantomData<fn() -> CV>,
}

impl<U, CV, E> AsyncEventBus<U, CV, E> {
    pub fn new(unblocker: U, event_bus: E) -> Self {
        Self {
            unblocker,
            event_bus,
            _condvar_type: PhantomData,
        }
    }
}

impl<U, CV, E> Clone for AsyncEventBus<U, CV, E>
where
    U: Clone,
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            unblocker: self.unblocker.clone(),
            event_bus: self.event_bus.clone(),
            _condvar_type: PhantomData,
        }
    }
}

impl<U, CV, E> super::UnblockingAsyncWrapper<U, E> for AsyncEventBus<U, CV, E> {
    fn new(unblocker: U, sync: E) -> Self {
        AsyncEventBus::new(unblocker, sync)
    }
}

impl<CV, E> super::AsyncWrapper<E> for AsyncEventBus<(), CV, E> {
    fn new(sync: E) -> Self {
        AsyncEventBus::new((), sync)
    }
}

impl<U, CV, E> ErrorType for AsyncEventBus<U, CV, E>
where
    E: ErrorType,
{
    type Error = E::Error;
}

impl<U, CV, P, E> EventBus<P> for AsyncEventBus<U, CV, E>
where
    CV: Condvar + Send + Sync + 'static,
    CV::Mutex<SubscriptionState<P, E::Subscription>>: Send + Sync + 'static,
    P: Clone + Send,
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
{
    type Subscription = AsyncSubscription<CV, P, E::Subscription>;

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

        let subscription = self.event_bus.subscribe(move |payload| {
            if let Some(state) = subscription_state.upgrade() {
                let pair: &(CV::Mutex<_>, CV) = &state;

                let (mut state, condvar) = (pair.0.lock(), &pair.1);

                while state.value.is_some() {
                    if let Some(waker) = mem::replace(&mut state.waker, None) {
                        waker.wake();
                    }

                    state = condvar.wait(state);
                }

                state.value = Some(payload.clone());

                if let Some(waker) = mem::replace(&mut state.waker, None) {
                    waker.wake();
                }
            }
        })?;

        state.0.lock().subscription = Some(subscription);

        Ok(AsyncSubscription(state))
    }
}

impl<U, CV, P, E> PostboxProvider<P> for AsyncEventBus<U, CV, E>
where
    U: Unblocker + Clone,
    CV: Condvar + Send + Sync + 'static,
    P: Clone + Send + 'static,
    E::Postbox: Clone + Send + 'static,
    E: crate::event_bus::PostboxProvider<P>,
    Self::Error: Send + Sync + 'static,
{
    type Postbox = AsyncPostbox<U, P, E::Postbox>;

    fn postbox(&mut self) -> Result<Self::Postbox, Self::Error> {
        self.event_bus
            .postbox()
            .map(|blocking_postbox| AsyncPostbox::new(self.unblocker.clone(), blocking_postbox))
    }
}

impl<CV, P, E> PostboxProvider<P> for AsyncEventBus<(), CV, E>
where
    CV: Condvar + Send + Sync + 'static,
    P: Clone + Send + 'static,
    E::Postbox: Clone + Send + 'static,
    E: crate::event_bus::PostboxProvider<P>,
    Self::Error: Send + Sync + 'static,
{
    type Postbox = AsyncPostbox<(), P, E::Postbox>;

    fn postbox(&mut self) -> Result<Self::Postbox, Self::Error> {
        self.event_bus
            .postbox()
            .map(|blocking_postbox| AsyncPostbox::new((), blocking_postbox))
    }
}
