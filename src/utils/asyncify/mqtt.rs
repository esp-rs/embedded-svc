pub mod client {
    use core::fmt::Debug;
    use core::future::Future;
    use core::mem;
    use core::pin::Pin;
    use core::task::{Context, Poll, Waker};

    extern crate alloc;
    use alloc::string::String;
    use alloc::sync::Arc;
    use alloc::vec::Vec;

    use crate::executor::asynch::Unblocker;
    use crate::mqtt::client::asynch::{Client, Connection, Event, MessageId, Publish, QoS};
    use crate::mqtt::client::utils::ConnStateGuard;
    use crate::mqtt::client::ErrorType;
    use crate::mutex::{RawCondvar, RawMutex};
    use crate::utils::mutex::Mutex;

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
        pub fn new(unblocker: U, client: W) -> Self {
            Self(client, unblocker)
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
        type SubscribeFuture<'a>
        where
            Self: 'a,
        = U::UnblockFuture<Result<MessageId, C::Error>>;

        type UnsubscribeFuture<'a>
        where
            Self: 'a,
        = U::UnblockFuture<Result<MessageId, C::Error>>;

        fn subscribe<'a>(&'a mut self, topic: &'a str, qos: QoS) -> Self::SubscribeFuture<'a> {
            let topic: String = topic.to_owned();
            let client = self.0.clone();

            self.1.unblock(move || client.lock().subscribe(&topic, qos))
        }

        fn unsubscribe<'a>(&'a mut self, topic: &'a str) -> Self::UnsubscribeFuture<'a> {
            let topic: String = topic.to_owned();
            let client = self.0.clone();

            self.1.unblock(move || client.lock().unsubscribe(&topic))
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
        type PublishFuture<'a>
        where
            Self: 'a,
        = U::UnblockFuture<Result<MessageId, C::Error>>;

        fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
        ) -> Self::PublishFuture<'a> {
            let topic: String = topic.to_owned();
            let payload: Vec<u8> = payload.to_owned();
            let client = self.0.clone();

            self.1
                .unblock(move || client.lock().publish(&topic, qos, retain, &payload))
        }
    }

    impl<U, R, C> crate::utils::asyncify::UnblockingAsyncWrapper<U, C>
        for AsyncClient<U, Arc<Mutex<R, C>>>
    where
        R: RawMutex,
    {
        fn new(unblocker: U, sync: C) -> Self {
            AsyncClient::new(unblocker, Arc::new(Mutex::new(sync)))
        }
    }

    pub struct Enqueueing;
    pub struct Publishing;

    pub struct Blocking<C, P> {
        client: C,
        _policy: P,
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
        type PublishFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<MessageId, E::Error>> + Send + 'a;

        fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
        ) -> Self::PublishFuture<'a> {
            enqueue_publish(&mut self.0.client, topic, qos, retain, payload)
        }
    }

    impl<P> Publish for AsyncClient<(), Blocking<P, Publishing>>
    where
        P: crate::mqtt::client::Publish + Send,
    {
        type PublishFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<MessageId, P::Error>> + Send + 'a;

        fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
        ) -> Self::PublishFuture<'a> {
            publish_publish(&mut self.0.client, topic, qos, retain, payload)
        }
    }

    impl<C, P> Client for AsyncClient<(), Blocking<C, P>>
    where
        C: crate::mqtt::client::Client + Send,
    {
        type SubscribeFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<MessageId, C::Error>> + Send + 'a;

        type UnsubscribeFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<MessageId, C::Error>> + Send + 'a;

        fn subscribe<'a>(&'a mut self, topic: &'a str, qos: QoS) -> Self::SubscribeFuture<'a> {
            client_subscribe(&mut self.0.client, topic, qos)
        }

        fn unsubscribe<'a>(&'a mut self, topic: &'a str) -> Self::UnsubscribeFuture<'a> {
            client_unsubscribe(&mut self.0.client, topic)
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

    pub enum AsyncConnState<M, E> {
        None,
        Waiting(Waker),
        Received(Result<Event<M>, E>),
    }

    impl<M, E> AsyncConnState<M, E> {
        pub fn new() -> Self {
            Self::None
        }
    }

    impl<M, E> Default for AsyncConnState<M, E> {
        fn default() -> Self {
            Self::new()
        }
    }

    pub struct NextFuture<'a, CV, M, E>(&'a ConnStateGuard<CV, AsyncConnState<M, E>>)
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
        pub fn new(connection_state: Arc<ConnStateGuard<CV, AsyncConnState<M, E>>>) -> Self {
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
        CV::RawMutex: Sync + 'static,
        M: Send,
        E: Debug + Send + 'static,
    {
        type Message = M;

        type NextFuture<'a>
        where
            Self: 'a,
            CV: 'a,
            M: 'a,
        = NextFuture<'a, CV, Self::Message, Self::Error>;

        fn next(&mut self) -> Self::NextFuture<'_> {
            NextFuture(&self.0)
        }
    }
}
