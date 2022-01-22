use std::{
    fmt::{Debug, Display},
    mem,
    stream::Stream,
    sync::{Arc, Condvar, Mutex},
    task::{Poll, Waker},
};

pub use crate::event_bus::Source;

pub struct SubscriptionState<E, P>
where
    E: crate::event_bus::EventBus,
    E::Subscription<P>: Send,
    P: Copy,
{
    subscription: Option<E::Subscription<P>>,
    value: Option<P>,
    waker: Option<Waker>,
}

pub struct Subscription<E, P>(Arc<(Mutex<SubscriptionState<E, P>>, Condvar)>)
where
    E: crate::event_bus::EventBus,
    E::Subscription<P>: Send,
    P: Copy;

impl<E, P> Stream for Subscription<E, P>
where
    E: crate::event_bus::EventBus,
    E::Subscription<P>: Send,
    P: Copy,
{
    type Item = Result<P, E::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut state = self.0 .0.lock().unwrap();

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

impl<E> crate::event_bus::nonblocking::EventBus for E
where
    E: crate::event_bus::EventBus + 'static,
{
    type SubscriptionStream<P>
    where
        P: Copy,
    = Subscription<E, P>;

    fn subscribe<P, ER>(
        &self,
        source: Source<P>,
    ) -> Result<Self::SubscriptionStream<P>, Self::Error>
    where
        P: Copy + Send + Sync + 'static,
        ER: Display + Debug + Send + Sync + 'static,
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

        let subscription = crate::event_bus::EventBus::subscribe(self, source, move |payload| {
            if let Some(state) = subscription_state.upgrade() {
                let (mut state, condvar) = (state.0.lock().unwrap(), &state.1);

                mem::replace(&mut state.waker, None).map(Waker::wake);

                while state.value.is_some() {
                    state = condvar.wait(state).unwrap();
                }

                state.value = Some(*payload);
            }

            Result::<_, Self::Error>::Ok(())
        })?;

        state.0.lock().unwrap().subscription = Some(subscription);

        Ok(Subscription(state))
    }
}
