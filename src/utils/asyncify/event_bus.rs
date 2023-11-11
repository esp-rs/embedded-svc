use core::fmt::Debug;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use core::time::Duration;

extern crate alloc;
use alloc::sync::Arc;

use crate::event_bus::ErrorType;
use crate::utils::asyncify::Unblocker;
use crate::utils::mutex::{Condvar, Mutex, RawCondvar};

#[allow(unused_imports)]
#[cfg(feature = "nightly")]
pub use async_traits_impl::*;

use super::{AsyncWrapper, UnblockingAsyncWrapper};

pub struct AsyncPostbox<U, P, PB> {
    unblocker: U,
    blocking_postbox: PB,
    _payload_type: PhantomData<fn() -> P>,
}

impl<U, P, PB> AsyncPostbox<U, P, PB> {
    pub const fn new(unblocker: U, blocking_postbox: PB) -> Self {
        Self {
            unblocker,
            blocking_postbox,
            _payload_type: PhantomData,
        }
    }
}

impl<P, PB> AsyncPostbox<(), P, PB> {
    pub fn send_blocking(
        &self,
        value: P,
        duration: Option<core::time::Duration>,
    ) -> Result<(), PB::Error>
    where
        PB: crate::event_bus::Postbox<P>,
    {
        self.blocking_postbox.post(&value, duration).map(|_| ())
    }
}

impl<U, P, PB> AsyncPostbox<U, P, PB>
where
    U: Unblocker,
{
    pub async fn send(&self, value: P) -> Result<(), PB::Error>
    where
        P: Send + 'static,
        PB: crate::event_bus::Postbox<P> + Sync,
        PB::Error: Send + 'static,
    {
        let blocking_postbox = &self.blocking_postbox;

        self.unblocker
            .unblock(move || {
                blocking_postbox
                    .post(&value, Some(Duration::MAX))
                    .map(|_| ())
            })
            .await
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

pub struct SubscriptionState<P, S> {
    subscription: Option<S>,
    value: Option<P>,
    waker: Option<Waker>,
}

#[allow(clippy::type_complexity)]
pub struct AsyncSubscription<CV, P, S, E>(
    Arc<(Mutex<CV::RawMutex, SubscriptionState<P, S>>, Condvar<CV>)>,
    PhantomData<fn() -> E>,
)
where
    CV: RawCondvar,
    P: Send,
    S: Send;

impl<CV, P, S, E> AsyncSubscription<CV, P, S, E>
where
    CV: RawCondvar + Send + Sync,
    CV::RawMutex: Send + Sync,
    S: Send,
    P: Clone + Send,
{
    pub async fn recv(&self) -> Result<P, E> {
        NextFuture(self).await
    }
}

struct NextFuture<'a, CV, P, S, E>(&'a AsyncSubscription<CV, P, S, E>)
where
    CV: RawCondvar + Send + Sync,
    CV::RawMutex: Send + Sync,
    P: Clone + Send,
    S: Send;

impl<'a, CV, P, S, E> Drop for NextFuture<'a, CV, P, S, E>
where
    CV: RawCondvar + Send + Sync,
    CV::RawMutex: Send + Sync,
    P: Clone + Send,
    S: Send,
{
    fn drop(&mut self) {
        let mut state = self.0 .0 .0.lock();
        state.waker = None;
    }
}

impl<'a, CV, P, S, E> Future for NextFuture<'a, CV, P, S, E>
where
    CV: RawCondvar + Send + Sync,
    CV::RawMutex: Send + Sync,
    P: Clone + Send,
    S: Send,
{
    type Output = Result<P, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0 .0 .0.lock();

        let value = state.value.take();

        if let Some(value) = value {
            self.0 .0 .1.notify_all();

            Poll::Ready(Ok(value))
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
    pub const fn new(unblocker: U, event_bus: E) -> Self {
        Self {
            unblocker,
            event_bus,
            _condvar_type: PhantomData,
        }
    }
}

#[allow(clippy::type_complexity)]
impl<U, CV, E> AsyncEventBus<U, CV, E>
where
    CV: RawCondvar + Send + Sync + 'static,
    CV::RawMutex: Send + Sync + 'static,
{
    pub fn subscribe<P>(
        &self,
    ) -> Result<AsyncSubscription<CV, P, E::Subscription<'_>, E::Error>, E::Error>
    where
        P: Clone + Send + 'static,
        E: crate::event_bus::EventBus<P>,
        for<'a> E::Subscription<'a>: Send + 'static,
    {
        let state = Arc::new((
            Mutex::new(SubscriptionState {
                subscription: None,
                value: None,
                waker: None,
            }),
            Condvar::new(),
        ));

        let subscription_state = Arc::downgrade(&state);

        let subscription = self.event_bus.subscribe(move |payload| {
            if let Some(state) = subscription_state.upgrade() {
                let pair: &(Mutex<CV::RawMutex, _>, Condvar<CV>) = &state;

                let (mut state, condvar) = (pair.0.lock(), &pair.1);

                while state.value.is_some() {
                    if let Some(waker) = state.waker.take() {
                        waker.wake();
                    }

                    state = condvar.wait(state);
                }

                state.value = Some(payload.clone());

                if let Some(waker) = state.waker.take() {
                    waker.wake();
                }
            }
        })?;

        state.0.lock().subscription = Some(subscription);

        Ok(AsyncSubscription(state, PhantomData))
    }
}

impl<U, CV, E> AsyncEventBus<U, CV, E>
where
    U: Clone,
    CV: RawCondvar + Send + Sync + 'static,
{
    pub fn postbox<P>(&self) -> Result<AsyncPostbox<U, P, E::Postbox<'_>>, E::Error>
    where
        E: crate::event_bus::PostboxProvider<P>,
    {
        self.event_bus
            .postbox()
            .map(|blocking_postbox| AsyncPostbox::new(self.unblocker.clone(), blocking_postbox))
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

impl<P, PB> AsyncWrapper<PB> for AsyncPostbox<(), P, PB> {
    fn new(sync: PB) -> Self {
        AsyncPostbox::new((), sync)
    }
}

impl<U, P, PB> UnblockingAsyncWrapper<U, PB> for AsyncPostbox<U, P, PB> {
    fn new(unblocker: U, sync: PB) -> Self {
        AsyncPostbox::new(unblocker, sync)
    }
}

impl<U, CV, E> UnblockingAsyncWrapper<U, E> for AsyncEventBus<U, CV, E> {
    fn new(unblocker: U, sync: E) -> Self {
        AsyncEventBus::new(unblocker, sync)
    }
}

impl<CV, E> AsyncWrapper<E> for AsyncEventBus<(), CV, E> {
    fn new(sync: E) -> Self {
        AsyncEventBus::new((), sync)
    }
}

impl<U, P, PB> ErrorType for AsyncPostbox<U, P, PB>
where
    PB: ErrorType,
{
    type Error = PB::Error;
}

impl<U, CV, E> ErrorType for AsyncEventBus<U, CV, E>
where
    E: ErrorType,
{
    type Error = E::Error;
}

impl<CV, P, S, E> ErrorType for AsyncSubscription<CV, P, S, E>
where
    CV: RawCondvar,
    P: Send,
    S: Send,
    E: Debug,
{
    type Error = E;
}

#[cfg(feature = "nightly")]
mod async_traits_impl {
    use core::fmt::Debug;

    use crate::event_bus::asynch::{EventBus, PostboxProvider, Receiver, Sender};
    use crate::utils::asyncify::Unblocker;
    use crate::utils::mutex::RawCondvar;

    use super::{AsyncEventBus, AsyncPostbox, AsyncSubscription};

    impl<U, P, PB> Sender for AsyncPostbox<U, P, PB>
    where
        U: Unblocker,
        P: Clone + Send + 'static,
        PB: crate::event_bus::Postbox<P> + Clone + Send + Sync,
        PB::Error: Send,
    {
        type Data = P;

        async fn send(&self, value: Self::Data) -> Result<(), Self::Error> {
            let value = value;
            let blocking_postbox = self.blocking_postbox.clone();

            self.unblocker
                .unblock(move || blocking_postbox.post(&value, None).map(|_| ()))
                .await
        }
    }

    impl<P, PB> Sender for AsyncPostbox<(), P, PB>
    where
        P: Send,
        PB: crate::event_bus::Postbox<P>,
    {
        type Data = P;

        async fn send(&self, value: Self::Data) -> Result<(), Self::Error> {
            AsyncPostbox::send_blocking(self, value, Some(core::time::Duration::MAX))
        }
    }

    impl<CV, P, S, E> Receiver for AsyncSubscription<CV, P, S, E>
    where
        CV: RawCondvar + Send + Sync,
        CV::RawMutex: Send + Sync,
        S: Send,
        P: Clone + Send,
        E: Debug,
    {
        type Data = P;

        async fn recv(&self) -> Result<Self::Data, Self::Error> {
            AsyncSubscription::recv(self).await
        }
    }

    impl<U, CV, P, E> EventBus<P> for AsyncEventBus<U, CV, E>
    where
        CV: RawCondvar + Send + Sync + 'static,
        CV::RawMutex: Send + Sync + 'static,
        P: Clone + Send + 'static,
        E: crate::event_bus::EventBus<P>,
        for<'a> E::Subscription<'a>: Send,
    {
        type Subscription<'a> = AsyncSubscription<CV, P, E::Subscription<'a>, E::Error> where Self: 'a;

        async fn subscribe(&self) -> Result<Self::Subscription<'_>, Self::Error> {
            AsyncEventBus::subscribe(self)
        }
    }

    impl<U, CV, P, E> PostboxProvider<P> for AsyncEventBus<U, CV, E>
    where
        U: Unblocker + Clone,
        CV: RawCondvar + Send + Sync + 'static,
        P: Clone + Send + 'static,
        for<'a> E::Postbox<'a>: Clone + Send + Sync,
        E: crate::event_bus::PostboxProvider<P>,
        Self::Error: Send + Sync + 'static,
    {
        type Postbox<'a> = AsyncPostbox<U, P, E::Postbox<'a>> where Self: 'a;

        async fn postbox(&self) -> Result<Self::Postbox<'_>, Self::Error> {
            AsyncEventBus::postbox(self)
        }
    }

    impl<CV, P, E> PostboxProvider<P> for AsyncEventBus<(), CV, E>
    where
        CV: RawCondvar + Send + Sync + 'static,
        P: Send + 'static,
        E: crate::event_bus::PostboxProvider<P>,
    {
        type Postbox<'a> = AsyncPostbox<(), P, E::Postbox<'a>> where Self: 'a;

        async fn postbox(&self) -> Result<Self::Postbox<'_>, Self::Error> {
            AsyncEventBus::postbox(self)
        }
    }
}
