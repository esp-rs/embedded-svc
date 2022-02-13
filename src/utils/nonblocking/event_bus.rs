use core::fmt::{Debug, Display};
use core::future::{ready, Future, Ready};
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

extern crate alloc;
use alloc::sync::Arc;

use crate::mutex::{Condvar, Mutex};

pub struct Postbox<P, PB> {
    blocking_postbox: PB,
    _payload_type: PhantomData<fn() -> P>,
}

impl<P, PB> Postbox<P, PB> {
    pub fn new(blocking_postbox: PB) -> Self {
        Self {
            blocking_postbox,
            _payload_type: PhantomData,
        }
    }
}

impl<P, PB> Clone for Postbox<P, PB>
where
    PB: Clone,
{
    fn clone(&self) -> Self {
        Self {
            blocking_postbox: self.blocking_postbox.clone(),
            _payload_type: PhantomData,
        }
    }
}

impl<P, PB> crate::service::Service for Postbox<P, PB>
where
    PB: crate::service::Service,
{
    type Error = PB::Error;
}

impl<P, PB> crate::channel::nonblocking::Sender for Postbox<P, PB>
where
    PB: crate::event_bus::Postbox<P>,
{
    type Data = P;

    type SendFuture<'a>
    where
        Self: 'a,
    = Ready<Result<(), Self::Error>>;

    fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
        // TODO: This will block if the queue is full.
        // Fix this by taking a notifier as to when the queue is
        // processed and awake the future when notified
        ready(self.blocking_postbox.post(&value, None).map(|_| ()))
    }
}

pub struct SubscriptionState<P, S> {
    subscription: Option<S>,
    value: Option<P>,
    waker: Option<Waker>,
}

#[allow(clippy::type_complexity)]
pub struct Subscription<CV, P, S, E>(
    Arc<(CV::Mutex<SubscriptionState<P, S>>, CV)>,
    PhantomData<fn() -> E>,
)
where
    CV: Condvar,
    P: Send,
    S: Send;

impl<CV, P, S, E> crate::service::Service for Subscription<CV, P, S, E>
where
    CV: Condvar,
    P: Send,
    S: Send,
    E: Display + Debug + Send + Sync + 'static,
{
    type Error = E;
}

impl<CV, P, S, E> crate::channel::nonblocking::Receiver for Subscription<CV, P, S, E>
where
    CV: Condvar,
    S: Send,
    P: Clone + Send,
    E: Display + Debug + Send + Sync + 'static,
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

pub struct NextFuture<'a, CV, P, S, E>(&'a Subscription<CV, P, S, E>)
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

pub struct Channel<CV, E> {
    blocking_channel: E,
    _condvar_type: PhantomData<fn() -> CV>,
}

impl<CV, E> Channel<CV, E> {
    pub fn new(blocking_channel: E) -> Self {
        Self {
            blocking_channel,
            _condvar_type: PhantomData,
        }
    }
}

impl<CV, E> Clone for Channel<CV, E>
where
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            blocking_channel: self.blocking_channel.clone(),
            _condvar_type: PhantomData,
        }
    }
}

impl<CV, E> super::AsyncWrapper<E> for Channel<CV, E> {
    fn new(sync: E) -> Self {
        Channel::new(sync)
    }
}

impl<CV, E> crate::service::Service for Channel<CV, E>
where
    E: crate::service::Service,
{
    type Error = E::Error;
}

impl<CV, P, E> crate::event_bus::nonblocking::EventBus<P> for Channel<CV, E>
where
    CV: Condvar + Send + Sync + 'static,
    CV::Mutex<SubscriptionState<P, E::Subscription>>: Send + Sync + 'static,
    P: Clone + Send,
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
{
    type Subscription = Subscription<CV, P, E::Subscription, E::Error>;

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
                let (mut state, condvar) = (state.0.lock(), &state.1);

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

        Ok(Subscription(state, PhantomData))
    }
}

impl<CV, P, E> crate::event_bus::nonblocking::PostboxProvider<P> for Channel<CV, E>
where
    CV: Condvar + Send + Sync + 'static,
    P: Clone + Send,
    E: crate::event_bus::PostboxProvider<P>,
{
    type Postbox = Postbox<P, E::Postbox>;

    fn postbox(&mut self) -> Result<Self::Postbox, Self::Error> {
        self.blocking_channel.postbox().map(Postbox::new)
    }
}
