use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use core::mem;
use core::stream::Stream;
use core::task::{Poll, Waker};

extern crate alloc;
use alloc::sync::Arc;

use crate::mutex::{Condvar, Mutex};

pub struct SubscriptionState<E>
where
    E: crate::event_bus::EventBus,
    E::Subscription: Send,
{
    subscription: Option<E::Subscription>,
    value: Option<E::Data>,
    waker: Option<Waker>,
}

pub struct Subscription<E, CV>(Arc<(CV::Mutex<SubscriptionState<E>>, CV)>)
where
    E: crate::event_bus::EventBus,
    E::Subscription: Send,
    CV: Condvar;

impl<E, CV> Stream for Subscription<E, CV>
where
    E: crate::event_bus::EventBus,
    E::Subscription: Send,
    CV: Condvar,
{
    type Item = Result<E::Data, E::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut state = self.0 .0.lock();

        if let Some(value) = state.value {
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

impl<E, CV> crate::event_bus::nonblocking::EventBus for EventBus<E, CV>
where
    E: crate::event_bus::EventBus + 'static,
    E::Subscription: Send,
    CV: Condvar + 'static,
{
    type Data = E::Data;

    type SubscriptionStream = Subscription<E, CV>;

    type Postbox = E::Postbox;

    fn subscribe<ER>(&self) -> Result<Self::SubscriptionStream, Self::Error>
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

                state.value = Some(*payload);
            }

            Result::<_, Self::Error>::Ok(())
        })?;

        state.0.lock().subscription = Some(subscription);

        Ok(Subscription(state))
    }

    fn postbox(&self) -> Result<Self::Postbox, Self::Error> {
        self.blocking_event_bus.postbox()
    }
}
