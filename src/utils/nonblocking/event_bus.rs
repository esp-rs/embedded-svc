use core::fmt::{Debug, Display};
use core::future::{ready, Future, Ready};
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

extern crate alloc;
use alloc::sync::Arc;

use crate::mutex::{Condvar, Mutex};

impl<PB, P> crate::event_bus::nonblocking::Postbox<P> for PB
where
    PB: crate::event_bus::Postbox<P>,
{
    type PostFuture<'a>
    where
        Self: 'a,
    = Ready<Result<(), Self::Error>>;

    fn post(&mut self, payload: P) -> Self::PostFuture<'_> {
        ready(crate::event_bus::Postbox::post(self, payload))
    }
}

pub struct SubscriptionState<E, P>
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
{
    subscription: Option<E::Subscription>,
    value: Option<P>,
    waker: Option<Waker>,
}

#[allow(clippy::type_complexity)]
pub struct Subscription<E, P, CV>(Arc<(CV::Mutex<SubscriptionState<E, P>>, CV)>)
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
    P: Send,
    CV: Condvar;

impl<E, P, CV> crate::service::Service for Subscription<E, P, CV>
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
    P: Clone + Send,
    CV: Condvar,
{
    type Error = E::Error;
}

impl<E, P, CV> crate::event_bus::nonblocking::Subscription<P> for Subscription<E, P, CV>
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
    P: Clone + Send,
    CV: Condvar,
{
    type NextFuture<'a>
    where
        Self: 'a,
    = NextFuture<'a, E, P, CV>;

    fn next(&mut self) -> Self::NextFuture<'_> {
        NextFuture(self)
    }
}

pub struct NextFuture<'a, E, P, CV>(&'a Subscription<E, P, CV>)
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
    P: Clone + Send,
    CV: Condvar;

impl<'a, E, P, CV> Drop for NextFuture<'a, E, P, CV>
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
    P: Clone + Send,
    CV: Condvar,
{
    fn drop(&mut self) {
        let mut state = self.0 .0 .0.lock();

        state.value = None;
        state.waker = None;
    }
}

impl<'a, E, P, CV> Future for NextFuture<'a, E, P, CV>
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
    P: Clone + Send,
    CV: Condvar,
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

pub struct EventBus<E, CV> {
    blocking_event_bus: E,
    _condvar_type: PhantomData<fn() -> CV>,
}

impl<E, CV> crate::service::Service for EventBus<E, CV>
where
    E: crate::service::Service,
{
    type Error = E::Error;
}

impl<E, P, CV> crate::event_bus::nonblocking::EventBus<P> for EventBus<E, CV>
where
    E: crate::event_bus::EventBus<P> + 'static,
    E::Subscription: Send,
    P: Clone + Send,
    CV: Condvar + Send + Sync + 'static,
    CV::Mutex<SubscriptionState<E, P>>: Send + Sync + 'static,
{
    type Subscription = Subscription<E, P, CV>;

    type Postbox = E::Postbox;

    fn subscribe<ER>(&mut self) -> Result<Self::Subscription, Self::Error>
    where
        ER: Display + Debug + Send + Sync + 'static,
    {
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
        self.blocking_event_bus.postbox()
    }
}
