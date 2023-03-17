use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

extern crate alloc;
use alloc::sync::Arc;

use crate::utils::mutex::{Condvar, Mutex, RawCondvar};

#[cfg(all(feature = "nightly", feature = "experimental"))]
pub use async_traits_impl::*;

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

    pub async fn send(&self, value: P)
    where
        P: Clone + Send + 'static,
        PB: crate::event_bus::Postbox<P> + Clone + Send + Sync + 'static,
    {
        self.blocking_postbox
            .post(&value, None)
            .map(|_| ())
            .unwrap()
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
pub struct AsyncSubscription<CV, P, S>(
    Arc<(Mutex<CV::RawMutex, SubscriptionState<P, S>>, Condvar<CV>)>,
)
where
    CV: RawCondvar,
    P: Send,
    S: Send;

impl<CV, P, S> AsyncSubscription<CV, P, S>
where
    CV: RawCondvar + Send + Sync,
    CV::RawMutex: Send + Sync,
    S: Send,
    P: Clone + Send,
{
    pub async fn recv(&mut self) -> P {
        NextFuture(self).await
    }
}

pub struct NextFuture<'a, CV, P, S>(&'a AsyncSubscription<CV, P, S>)
where
    CV: RawCondvar + Send + Sync,
    CV::RawMutex: Send + Sync,
    P: Clone + Send,
    S: Send;

impl<'a, CV, P, S> Drop for NextFuture<'a, CV, P, S>
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

impl<'a, CV, P, S> Future for NextFuture<'a, CV, P, S>
where
    CV: RawCondvar + Send + Sync,
    CV::RawMutex: Send + Sync,
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
    pub const fn new(unblocker: U, event_bus: E) -> Self {
        Self {
            unblocker,
            event_bus,
            _condvar_type: PhantomData,
        }
    }
}

impl<U, CV, E> AsyncEventBus<U, CV, E>
where
    CV: RawCondvar + Send + Sync + 'static,
    CV::RawMutex: Send + Sync + 'static,
{
    pub fn subscribe<P>(&self) -> Result<AsyncSubscription<CV, P, E::Subscription>, E::Error>
    where
        P: Clone + Send + 'static,
        E: crate::event_bus::EventBus<P>,
        E::Subscription: Send + 'static,
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

impl<CV, E> AsyncEventBus<(), CV, E>
where
    CV: RawCondvar + Send + Sync + 'static,
{
    pub fn postbox<P>(&self) -> Result<AsyncPostbox<(), P, E::Postbox>, E::Error>
    where
        P: Clone + Send + 'static,
        E::Postbox: Clone + Send + 'static,
        E: crate::event_bus::PostboxProvider<P>,
        E::Error: Send + Sync + 'static,
    {
        self.event_bus
            .postbox()
            .map(|blocking_postbox| AsyncPostbox::new((), blocking_postbox))
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

#[cfg(all(feature = "nightly", feature = "experimental"))]
mod async_traits_impl {
    use core::future::Future;

    use crate::event_bus::asynch::{ErrorType, EventBus, PostboxProvider, Receiver, Sender};
    use crate::executor::asynch::Unblocker;
    use crate::utils::asyncify::{AsyncWrapper, UnblockingAsyncWrapper};
    use crate::utils::mutex::RawCondvar;

    use super::{AsyncEventBus, AsyncPostbox, AsyncSubscription, NextFuture};

    impl<U, P, PB> Sender for AsyncPostbox<U, P, PB>
    where
        U: Unblocker,
        P: Clone + Send + 'static,
        PB: crate::event_bus::Postbox<P> + Clone + Send + Sync + 'static,
    {
        type Data = P;

        type SendFuture<'a>
        = U::UnblockFuture<()> where Self: 'a;

        fn send(&self, value: Self::Data) -> Self::SendFuture<'_> {
            let value = value;
            let blocking_postbox = self.blocking_postbox.clone();

            self.unblocker
                .unblock(move || blocking_postbox.post(&value, None).map(|_| ()).unwrap())
        }
    }

    impl<P, PB> Sender for AsyncPostbox<(), P, PB>
    where
        P: Clone + Send + 'static,
        PB: crate::event_bus::Postbox<P> + Clone + Send + Sync + 'static,
    {
        type Data = P;

        type SendFuture<'a>
        = impl Future<Output = ()> + 'a where Self: 'a;

        fn send(&self, value: Self::Data) -> Self::SendFuture<'_> {
            AsyncPostbox::send(self, value)
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

    impl<CV, P, S> Receiver for AsyncSubscription<CV, P, S>
    where
        CV: RawCondvar + Send + Sync,
        CV::RawMutex: Send + Sync,
        S: Send,
        P: Clone + Send,
    {
        type Data = P;

        type RecvFuture<'a>
        = NextFuture<'a, CV, P, S> where Self: 'a;

        fn recv(&self) -> Self::RecvFuture<'_> {
            NextFuture(self)
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

    impl<U, CV, E> ErrorType for AsyncEventBus<U, CV, E>
    where
        E: ErrorType,
    {
        type Error = E::Error;
    }

    impl<U, CV, P, E> EventBus<P> for AsyncEventBus<U, CV, E>
    where
        CV: RawCondvar + Send + Sync + 'static,
        CV::RawMutex: Send + Sync + 'static,
        P: Clone + Send + 'static,
        E: crate::event_bus::EventBus<P>,
        E::Subscription: Send + 'static,
    {
        type Subscription = AsyncSubscription<CV, P, E::Subscription>;

        fn subscribe(&self) -> Result<Self::Subscription, Self::Error> {
            AsyncEventBus::subscribe(self)
        }
    }

    impl<U, CV, P, E> PostboxProvider<P> for AsyncEventBus<U, CV, E>
    where
        U: Unblocker + Clone,
        CV: RawCondvar + Send + Sync + 'static,
        P: Clone + Send + 'static,
        E::Postbox: Clone + Send + Sync + 'static,
        E: crate::event_bus::PostboxProvider<P>,
        Self::Error: Send + Sync + 'static,
    {
        type Postbox = AsyncPostbox<U, P, E::Postbox>;

        fn postbox(&self) -> Result<Self::Postbox, Self::Error> {
            self.event_bus
                .postbox()
                .map(|blocking_postbox| AsyncPostbox::new(self.unblocker.clone(), blocking_postbox))
        }
    }

    impl<CV, P, E> PostboxProvider<P> for AsyncEventBus<(), CV, E>
    where
        CV: RawCondvar + Send + Sync + 'static,
        P: Clone + Send + 'static,
        E::Postbox: Clone + Send + Sync + 'static,
        E: crate::event_bus::PostboxProvider<P>,
        Self::Error: Send + Sync + 'static,
    {
        type Postbox = AsyncPostbox<(), P, E::Postbox>;

        fn postbox(&self) -> Result<Self::Postbox, Self::Error> {
            AsyncEventBus::postbox(self)
        }
    }
}
