pub mod client {
    use core::fmt::Debug;
    use core::future::Future;
    use core::mem;
    use core::pin::Pin;
    use core::task::{Context, Poll, Waker};

    extern crate alloc;
    use alloc::sync::Arc;

    use crate::mqtt::client::{Event, MessageId, QoS};
    use crate::utils::mqtt::client::ConnStateGuard;
    use crate::utils::mutex::{Mutex, RawCondvar, RawMutex};

    #[allow(unused_imports)]
    #[cfg(feature = "nightly")]
    pub use async_traits_impl::*;

    async fn enqueue_publish<'a, E>(
        enqueue: &'a mut E,
        topic: &'a str,
        qos: QoS,
        retain: bool,
        payload: &'a [u8],
    ) -> Result<MessageId, E::Error>
    where
        E: crate::mqtt::client::Enqueue + 'a,
    {
        enqueue.enqueue(topic, qos, retain, payload)
    }

    async fn publish_publish<'a, P>(
        publish: &'a mut P,
        topic: &'a str,
        qos: QoS,
        retain: bool,
        payload: &'a [u8],
    ) -> Result<MessageId, P::Error>
    where
        P: crate::mqtt::client::Publish + 'a,
    {
        publish.publish(topic, qos, retain, payload)
    }

    async fn client_subscribe<'a, C>(
        client: &'a mut C,
        topic: &'a str,
        qos: QoS,
    ) -> Result<MessageId, C::Error>
    where
        C: crate::mqtt::client::Client + 'a,
    {
        client.subscribe(topic, qos)
    }

    async fn client_unsubscribe<'a, C>(
        client: &'a mut C,
        topic: &'a str,
    ) -> Result<MessageId, C::Error>
    where
        C: crate::mqtt::client::Client + 'a,
    {
        client.unsubscribe(topic)
    }

    pub struct AsyncClient<U, W>(W, U);

    impl<U, W> AsyncClient<U, W> {
        pub const fn new(unblocker: U, client: W) -> Self {
            Self(client, unblocker)
        }
    }

    pub struct Enqueueing;
    pub struct Publishing;

    pub struct Blocking<C, P> {
        client: C,
        _policy: P,
    }

    impl<E> AsyncClient<(), Blocking<E, Enqueueing>>
    where
        E: crate::mqtt::client::Enqueue + Send,
    {
        pub async fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
        ) -> Result<MessageId, E::Error> {
            enqueue_publish(&mut self.0.client, topic, qos, retain, payload).await
        }
    }

    impl<P> AsyncClient<(), Blocking<P, Publishing>>
    where
        P: crate::mqtt::client::Publish + Send,
    {
        pub async fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
        ) -> Result<MessageId, P::Error> {
            publish_publish(&mut self.0.client, topic, qos, retain, payload).await
        }
    }

    impl<C, P> AsyncClient<(), Blocking<C, P>>
    where
        C: crate::mqtt::client::Client + Send,
    {
        pub async fn subscribe<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
        ) -> Result<MessageId, C::Error> {
            client_subscribe(&mut self.0.client, topic, qos).await
        }

        pub async fn unsubscribe<'a>(&'a mut self, topic: &'a str) -> Result<MessageId, C::Error> {
            client_unsubscribe(&mut self.0.client, topic).await
        }
    }

    impl<C> AsyncClient<(), Blocking<C, Publishing>> {
        pub fn into_enqueueing(self) -> AsyncClient<(), Blocking<C, Enqueueing>> {
            AsyncClient::new(
                (),
                Blocking {
                    client: self.0.client,
                    _policy: Enqueueing,
                },
            )
        }
    }

    impl<C> AsyncClient<(), Blocking<C, Enqueueing>> {
        pub fn into_publishing(self) -> AsyncClient<(), Blocking<C, Publishing>> {
            AsyncClient::new(
                (),
                Blocking {
                    client: self.0.client,
                    _policy: Publishing,
                },
            )
        }
    }

    pub enum AsyncConnState<M, E> {
        None,
        Waiting(Waker),
        Received(Result<Event<M>, E>),
    }

    impl<M, E> AsyncConnState<M, E> {
        pub const fn new() -> Self {
            Self::None
        }
    }

    impl<M, E> Default for AsyncConnState<M, E> {
        fn default() -> Self {
            Self::new()
        }
    }

    struct NextFuture<'a, CV, M, E>(&'a ConnStateGuard<CV, AsyncConnState<M, E>>)
    where
        CV: RawCondvar + 'a,
        M: 'a,
        E: 'a;

    impl<'a, CV, M, E> Future for NextFuture<'a, CV, M, E>
    where
        CV: RawCondvar + 'a,
        M: 'a,
        E: 'a,
    {
        type Output = Option<Result<Event<M>, E>>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut state = self.0.state.lock();

            if let Some(state) = &mut *state {
                let pulled = mem::replace(state, AsyncConnState::None);

                match pulled {
                    AsyncConnState::Received(event) => {
                        self.0.state_changed.notify_all();

                        Poll::Ready(Some(event))
                    }
                    _ => {
                        *state = AsyncConnState::Waiting(cx.waker().clone());
                        self.0.state_changed.notify_all();

                        Poll::Pending
                    }
                }
            } else {
                Poll::Ready(None)
            }
        }
    }

    pub struct AsyncPostbox<CV, M, E>(Arc<ConnStateGuard<CV, AsyncConnState<M, E>>>)
    where
        CV: RawCondvar;

    impl<CV, M, E> AsyncPostbox<CV, M, E>
    where
        CV: RawCondvar,
        M: Send,
        E: Send,
    {
        pub const fn new(connection_state: Arc<ConnStateGuard<CV, AsyncConnState<M, E>>>) -> Self {
            Self(connection_state)
        }

        pub fn post(&mut self, event: Result<Event<M>, E>) {
            let mut state = self.0.state.lock();

            loop {
                if state.is_none() {
                    return;
                } else if matches!(&*state, Some(AsyncConnState::Received(_))) {
                    state = self.0.state_changed.wait(state);
                } else {
                    break;
                }
            }

            if let Some(AsyncConnState::Waiting(waker)) =
                mem::replace(&mut *state, Some(AsyncConnState::Received(event)))
            {
                waker.wake();
            }
        }
    }

    pub struct AsyncConnection<CV, M, E>(Arc<ConnStateGuard<CV, AsyncConnState<M, E>>>)
    where
        CV: RawCondvar;

    impl<CV, M, E> AsyncConnection<CV, M, E>
    where
        CV: RawCondvar,
    {
        pub fn new(connection_state: Arc<ConnStateGuard<CV, AsyncConnState<M, E>>>) -> Self {
            Self(connection_state)
        }
    }

    impl<CV, M, E> Drop for AsyncConnection<CV, M, E>
    where
        CV: RawCondvar,
    {
        fn drop(&mut self) {
            self.0.close();
        }
    }

    impl<CV, M, E> AsyncConnection<CV, M, E>
    where
        CV: RawCondvar + Send + Sync + 'static,
        CV::RawMutex: Sync + 'static,
        M: Send,
        E: Debug + Send + 'static,
    {
        #[allow(clippy::should_implement_trait)]
        pub async fn next(&mut self) -> Option<Result<Event<M>, E>> {
            NextFuture(&self.0).await
        }
    }

    impl<C> crate::utils::asyncify::AsyncWrapper<C> for AsyncClient<(), Blocking<C, Publishing>> {
        fn new(sync: C) -> Self {
            AsyncClient::new(
                (),
                Blocking {
                    client: sync,
                    _policy: Publishing,
                },
            )
        }
    }

    #[allow(clippy::arc_with_non_send_sync)]
    impl<U, R, C> crate::utils::asyncify::UnblockingAsyncWrapper<U, C>
        for AsyncClient<U, Arc<Mutex<R, C>>>
    where
        R: RawMutex,
    {
        fn new(unblocker: U, sync: C) -> Self {
            AsyncClient::new(unblocker, Arc::new(Mutex::new(sync)))
        }
    }

    #[cfg(feature = "nightly")]
    mod async_traits_impl {
        use core::fmt::Debug;

        extern crate alloc;
        use alloc::borrow::ToOwned;
        use alloc::string::String;
        use alloc::sync::Arc;
        use alloc::vec::Vec;

        use crate::mqtt::client::asynch::{Client, Connection, MessageId, Publish, QoS};
        use crate::mqtt::client::ErrorType;
        use crate::utils::asyncify::mqtt::client::Event;
        use crate::utils::asyncify::Unblocker;
        use crate::utils::mutex::{Mutex, RawCondvar, RawMutex};

        use super::{
            client_subscribe, client_unsubscribe, enqueue_publish, publish_publish, AsyncClient,
            AsyncConnection, Blocking, Enqueueing, Publishing,
        };

        impl<U, R, C> ErrorType for AsyncClient<U, Arc<Mutex<R, C>>>
        where
            R: RawMutex,
            C: ErrorType,
        {
            type Error = C::Error;
        }

        impl<U, R, C> Client for AsyncClient<U, Arc<Mutex<R, C>>>
        where
            U: Unblocker,
            R: RawMutex + Send + Sync + 'static,
            C: crate::mqtt::client::Client + Send + 'static,
            C::Error: Clone,
            Self::Error: Send + Sync + 'static,
        {
            async fn subscribe(&mut self, topic: &str, qos: QoS) -> Result<MessageId, C::Error> {
                let topic: String = topic.to_owned();
                let client = self.0.clone();

                self.1
                    .unblock(move || client.lock().subscribe(&topic, qos))
                    .await
            }

            async fn unsubscribe(&mut self, topic: &str) -> Result<MessageId, C::Error> {
                let topic: String = topic.to_owned();
                let client = self.0.clone();

                self.1
                    .unblock(move || client.lock().unsubscribe(&topic))
                    .await
            }
        }

        impl<U, R, C> Publish for AsyncClient<U, Arc<Mutex<R, C>>>
        where
            U: Unblocker,
            R: RawMutex + Send + Sync + 'static,
            C: crate::mqtt::client::Publish + Send + 'static,
            C::Error: Clone,
            Self::Error: Send + Sync + 'static,
        {
            async fn publish(
                &mut self,
                topic: &str,
                qos: QoS,
                retain: bool,
                payload: &[u8],
            ) -> Result<MessageId, C::Error> {
                let topic: String = topic.to_owned();
                let payload: Vec<u8> = payload.to_owned();
                let client = self.0.clone();

                self.1
                    .unblock(move || client.lock().publish(&topic, qos, retain, &payload))
                    .await
            }
        }

        impl<E, P> ErrorType for AsyncClient<(), Blocking<E, P>>
        where
            E: ErrorType,
        {
            type Error = E::Error;
        }

        impl<E> Publish for AsyncClient<(), Blocking<E, Enqueueing>>
        where
            E: crate::mqtt::client::Enqueue + Send,
        {
            async fn publish(
                &mut self,
                topic: &str,
                qos: QoS,
                retain: bool,
                payload: &[u8],
            ) -> Result<MessageId, E::Error> {
                enqueue_publish(&mut self.0.client, topic, qos, retain, payload).await
            }
        }

        impl<P> Publish for AsyncClient<(), Blocking<P, Publishing>>
        where
            P: crate::mqtt::client::Publish + Send,
        {
            async fn publish(
                &mut self,
                topic: &str,
                qos: QoS,
                retain: bool,
                payload: &[u8],
            ) -> Result<MessageId, P::Error> {
                publish_publish(&mut self.0.client, topic, qos, retain, payload).await
            }
        }

        impl<C, P> Client for AsyncClient<(), Blocking<C, P>>
        where
            C: crate::mqtt::client::Client + Send,
        {
            async fn subscribe(&mut self, topic: &str, qos: QoS) -> Result<MessageId, C::Error> {
                client_subscribe(&mut self.0.client, topic, qos).await
            }

            async fn unsubscribe(&mut self, topic: &str) -> Result<MessageId, C::Error> {
                client_unsubscribe(&mut self.0.client, topic).await
            }
        }

        impl<CV, M, E> ErrorType for AsyncConnection<CV, M, E>
        where
            CV: RawCondvar,
            E: Debug,
        {
            type Error = E;
        }

        impl<CV, M, E> Connection for AsyncConnection<CV, M, E>
        where
            CV: RawCondvar + Send + Sync + 'static,
            CV::RawMutex: Send + Sync + 'static,
            M: Send,
            E: Debug + Send + 'static,
        {
            type Message<'a> = M where Self: 'a;

            async fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>> {
                AsyncConnection::next(self).await
            }
        }
    }
}
