use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use core::mem;
use core::stream::Stream;
use core::task::{Poll, Waker};

extern crate alloc;
use alloc::sync::Arc;

use crate::mutex::{Condvar, Mutex};

pub struct SubscriptionState<E, P>
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
{
    subscription: Option<E::Subscription>,
    value: Option<P>,
    waker: Option<Waker>,
}

pub struct Subscription<E, P, CV>(Arc<(CV::Mutex<SubscriptionState<E, P>>, CV)>)
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
    P: Send,
    CV: Condvar;

impl<E, P, CV> Stream for Subscription<E, P, CV>
where
    E: crate::event_bus::EventBus<P>,
    E::Subscription: Send,
    P: Clone + Send,
    CV: Condvar,
{
    type Item = Result<P, E::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut state = self.0 .0.lock();

        if let Some(value) = state.value.as_ref() {
            let value = value.clone();

            state.value = None;

            self.0 .1.notify_all();

            Poll::Ready(Some(Ok(value)))
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
    type SubscriptionStream = Subscription<E, P, CV>;

    type Postbox = E::Postbox;

    fn subscribe<ER>(&mut self) -> Result<Self::SubscriptionStream, Self::Error>
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

                mem::replace(&mut state.waker, None).map(Waker::wake);

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
