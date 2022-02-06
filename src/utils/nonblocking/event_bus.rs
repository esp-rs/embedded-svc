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

impl<P, PB> Postbox<P, PB>
where
    PB: crate::event_bus::Postbox<P>,
{
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

pub struct SubscriptionState<P, E>
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
{
    subscription: Option<E::Subscription>,
    value: Option<P>,
    waker: Option<Waker>,
}

#[allow(clippy::type_complexity)]
pub struct Subscription<CV, P, E>(Arc<(CV::Mutex<SubscriptionState<P, E>>, CV)>)
where
    CV: Condvar,
    P: Send,
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send;

impl<CV, P, E> crate::service::Service for Subscription<CV, P, E>
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
    P: Clone + Send,
    CV: Condvar,
{
    type Error = E::Error;
}

impl<CV, P, E> crate::channel::nonblocking::Receiver for Subscription<CV, P, E>
where
    CV: Condvar,
    P: Clone + Send,
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
{
    type Data = P;

    type RecvFuture<'a>
    where
        Self: 'a,
    = NextFuture<'a, CV, P, E>;

    fn recv(&mut self) -> Self::RecvFuture<'_> {
        NextFuture(self)
    }
}

pub struct NextFuture<'a, CV, P, E>(&'a Subscription<CV, P, E>)
where
    CV: Condvar,
    P: Clone + Send,
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send;

impl<'a, CV, P, E> Drop for NextFuture<'a, CV, P, E>
where
    CV: Condvar,
    P: Clone + Send,
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
{
    fn drop(&mut self) {
        let mut state = self.0 .0 .0.lock();

        state.value = None;
        state.waker = None;
    }
}

impl<'a, CV, P, E> Future for NextFuture<'a, CV, P, E>
where
    CV: Condvar,
    P: Clone + Send,
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
{
    type Output = Result<P, E::Error>;

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

pub struct EventBus<CV, E> {
    blocking_event_bus: E,
    _condvar_type: PhantomData<fn() -> CV>,
}

impl<CV, E> EventBus<CV, E> {
    pub fn new<P>(blocking_event_bus: E) -> Self
    where
        E: crate::event_bus::EventBus<P>,
    {
        Self {
            blocking_event_bus,
            _condvar_type: PhantomData,
        }
    }
}

impl<CV, E> Clone for EventBus<CV, E>
where
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            blocking_event_bus: self.blocking_event_bus.clone(),
            _condvar_type: PhantomData,
        }
    }
}

impl<CV, E> crate::service::Service for EventBus<CV, E>
where
    E: crate::service::Service,
{
    type Error = E::Error;
}

impl<CV, P, E> crate::event_bus::nonblocking::EventBus<P> for EventBus<CV, E>
where
    CV: Condvar + Send + Sync + 'static,
    CV::Mutex<SubscriptionState<P, E>>: Send + Sync + 'static,
    P: Clone + Send,
    E: crate::event_bus::EventBus<P> + 'static,
    E::Subscription: Send,
{
    type Subscription = Subscription<CV, P, E>;

    type Postbox = Postbox<P, E::Postbox>;

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

        let subscription = self.blocking_event_bus.subscribe(move |payload| {
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

            Result::<_, Self::Error>::Ok(())
        })?;

        state.0.lock().subscription = Some(subscription);

        Ok(Subscription(state))
    }

    fn postbox(&mut self) -> Result<Self::Postbox, Self::Error> {
        self.blocking_event_bus.postbox().map(Postbox::new)
    }
}
