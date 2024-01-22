pub mod client {
    use core::fmt::Debug;
    use core::marker::PhantomData;

    extern crate alloc;
    use alloc::borrow::ToOwned;
    use alloc::string::String;
    use alloc::sync::Arc;
    use alloc::vec::Vec;

    use crate::mqtt::client::asynch::{Client, Connection, Publish};
    use crate::mqtt::client::{ErrorType, Event, MessageId, QoS};

    use crate::utils::asyncify::Unblocker;
    use crate::utils::mutex::{Mutex, RawCondvar, RawMutex};
    use crate::utils::zerocopy::Receiver;

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

    pub struct AsyncConnection<C, E, X>
    where
        C: RawCondvar,
    {
        receiver: Receiver<C, E>,
        given: bool,
        _error: PhantomData<fn() -> X>,
    }

    impl<C, E, X> AsyncConnection<C, E, X>
    where
        C: RawCondvar,
    {
        pub const fn new(receiver: Receiver<C, E>) -> Self {
            Self {
                receiver,
                given: false,
                _error: PhantomData,
            }
        }

        pub async fn next(&mut self) -> Result<Option<&mut E>, X> {
            if self.given {
                self.receiver.done();
            } else {
                self.given = true;
            }

            Ok(self.receiver.get_async().await)
        }
    }

    impl<C, E, X> ErrorType for AsyncConnection<C, E, X>
    where
        C: RawCondvar,
        X: Debug,
    {
        type Error = X;
    }

    impl<C, E, X> Connection for AsyncConnection<C, E, X>
    where
        C: RawCondvar,
        E: Event,
        X: Debug,
    {
        type Event<'a> = &'a E where Self: 'a;

        async fn next(&mut self) -> Result<Option<Self::Event<'_>>, Self::Error> {
            if let Some(data) = AsyncConnection::next(self).await? {
                Ok(Some(data))
            } else {
                Ok(None)
            }
        }
    }
}
