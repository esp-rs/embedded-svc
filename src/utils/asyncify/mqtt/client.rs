use core::future::Future;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

extern crate alloc;
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::errors::{self, Errors};
use crate::mqtt::client::asyncs::{Client, Connection, Event, MessageId, Publish, QoS};
use crate::mqtt::client::utils::ConnectionState;
use crate::mutex::{Condvar, Mutex, MutexFamily};
use crate::unblocker::asyncs::Unblocker;

pub trait PublishPolicy {}

pub struct Enqueueing;

impl PublishPolicy for () {}
impl PublishPolicy for Enqueueing {}

fn enqueue_publish<'a, E>(
    enqueue: &'a mut E,
    topic: Cow<'a, str>,
    qos: QoS,
    retain: bool,
    payload: Cow<'a, [u8]>,
) -> impl Future<Output = Result<MessageId, E::Error>> + 'a
where
    E: crate::mqtt::client::Enqueue + 'a,
{
    async move { enqueue.enqueue(topic, qos, retain, payload) }
}

fn publish_publish<'a, P>(
    publish: &'a mut P,
    topic: Cow<'a, str>,
    qos: QoS,
    retain: bool,
    payload: Cow<'a, [u8]>,
) -> impl Future<Output = Result<MessageId, P::Error>> + 'a
where
    P: crate::mqtt::client::Publish + 'a,
{
    async move { publish.publish(topic, qos, retain, payload) }
}

fn client_subscribe<'a, C>(
    client: &'a mut C,
    topic: Cow<'a, str>,
    qos: QoS,
) -> impl Future<Output = Result<MessageId, C::Error>> + 'a
where
    C: crate::mqtt::client::Client + 'a,
{
    async move { client.subscribe(topic, qos) }
}

fn client_unsubscribe<'a, C>(
    client: &'a mut C,
    topic: Cow<'a, str>,
) -> impl Future<Output = Result<MessageId, C::Error>> + 'a
where
    C: crate::mqtt::client::Client + 'a,
{
    async move { client.unsubscribe(topic) }
}

pub struct AsyncClient<U, W>(W, U);

impl<U, W> AsyncClient<U, W> {
    pub fn new(unblocker: U, client: W) -> Self {
        Self(client, unblocker)
    }
}

impl<U, M, C> Errors for AsyncClient<U, Arc<M>>
where
    M: Mutex<Data = C>,
    C: Errors,
{
    type Error = C::Error;
}

impl<U, M, C> Client for AsyncClient<U, Arc<M>>
where
    U: Unblocker,
    M: Mutex<Data = C> + Send + Sync + 'static,
    C: crate::mqtt::client::Client,
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

    fn subscribe<'a, S>(&'a mut self, topic: S, qos: QoS) -> Self::SubscribeFuture<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        let topic: String = topic.into().into_owned();
        let client = self.0.clone();

        self.1.unblock(move || client.lock().subscribe(&topic, qos))
    }

    fn unsubscribe<'a, S>(&'a mut self, topic: S) -> Self::UnsubscribeFuture<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        let topic: String = topic.into().into_owned();
        let client = self.0.clone();

        self.1.unblock(move || client.lock().unsubscribe(&topic))
    }
}

impl<U, M, C> Publish for AsyncClient<U, Arc<M>>
where
    U: Unblocker,
    M: Mutex<Data = C> + Send + Sync + 'static,
    C: crate::mqtt::client::Publish,
    C::Error: Clone,
    Self::Error: Send + Sync + 'static,
{
    type PublishFuture<'a>
    where
        Self: 'a,
    = U::UnblockFuture<Result<MessageId, C::Error>>;

    fn publish<'a, S, V>(
        &'a mut self,
        topic: S,
        qos: QoS,
        retain: bool,
        payload: V,
    ) -> Self::PublishFuture<'a>
    where
        S: Into<Cow<'a, str>>,
        V: Into<Cow<'a, [u8]>>,
    {
        let topic: String = topic.into().into_owned();
        let payload: Vec<u8> = payload.into().into_owned();
        let client = self.0.clone();

        self.1
            .unblock(move || client.lock().publish(&topic, qos, retain, &payload))
    }
}

impl<U, M, C> crate::utils::asyncify::UnblockingAsyncWrapper<U, C> for AsyncClient<U, Arc<M>>
where
    M: Mutex<Data = C>,
{
    fn new(unblocker: U, sync: C) -> Self {
        AsyncClient::new(unblocker, Arc::new(M::new(sync)))
    }
}

impl<U, E> Errors for AsyncClient<U, E>
where
    E: Errors,
{
    type Error = E::Error;
}

impl<E> Publish for AsyncClient<Enqueueing, E>
where
    E: crate::mqtt::client::Enqueue + Send,
{
    type PublishFuture<'a>
    where
        Self: 'a,
    = impl Future<Output = Result<MessageId, E::Error>> + Send + 'a;

    fn publish<'a, S, V>(
        &'a mut self,
        topic: S,
        qos: QoS,
        retain: bool,
        payload: V,
    ) -> Self::PublishFuture<'a>
    where
        S: Into<Cow<'a, str>>,
        V: Into<Cow<'a, [u8]>>,
    {
        enqueue_publish(&mut self.0, topic.into(), qos, retain, payload.into())
    }
}

impl<P> Publish for AsyncClient<(), P>
where
    P: crate::mqtt::client::Publish + Send,
{
    type PublishFuture<'a>
    where
        Self: 'a,
    = impl Future<Output = Result<MessageId, P::Error>> + Send + 'a;

    fn publish<'a, S, V>(
        &'a mut self,
        topic: S,
        qos: QoS,
        retain: bool,
        payload: V,
    ) -> Self::PublishFuture<'a>
    where
        S: Into<Cow<'a, str>>,
        V: Into<Cow<'a, [u8]>>,
    {
        publish_publish(&mut self.0, topic.into(), qos, retain, payload.into())
    }
}

impl<U, C> Client for AsyncClient<U, C>
where
    U: PublishPolicy,
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

    fn subscribe<'a, S>(&'a mut self, topic: S, qos: QoS) -> Self::SubscribeFuture<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        client_subscribe(&mut self.0, topic.into(), qos)
    }

    fn unsubscribe<'a, S>(&'a mut self, topic: S) -> Self::UnsubscribeFuture<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        client_unsubscribe(&mut self.0, topic.into())
    }
}

impl<C> AsyncClient<(), C> {
    pub fn into_enqueueing(self) -> AsyncClient<Enqueueing, C> {
        AsyncClient::new(Enqueueing, self.0)
    }
}

impl<C> AsyncClient<Enqueueing, C> {
    pub fn into_publishing(self) -> AsyncClient<(), C> {
        AsyncClient::new((), self.0)
    }
}

impl<C> crate::utils::asyncify::AsyncWrapper<C> for AsyncClient<(), C> {
    fn new(sync: C) -> Self {
        AsyncClient::new((), sync)
    }
}

pub enum AsyncState<R, E> {
    None,
    Waiting(Waker),
    Received(Result<Event<R>, E>),
}

impl<R, E> AsyncState<R, E> {
    pub fn new() -> Self {
        Self::None
    }
}

impl<R, E> Default for AsyncState<R, E> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NextFuture<'a, CV, R, E>(&'a ConnectionState<CV, AsyncState<R, E>>)
where
    CV: Condvar + 'a,
    R: 'a,
    E: 'a;

impl<'a, CV, R, E> Future for NextFuture<'a, CV, R, E>
where
    CV: Condvar + 'a,
    R: 'a,
    E: 'a,
{
    type Output = Option<Result<Event<R>, E>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.state.lock();

        if let Some(state) = &mut *state {
            let pulled = mem::replace(state, AsyncState::None);

            match pulled {
                AsyncState::Received(event) => {
                    self.0.state_changed.notify_all();

                    Poll::Ready(Some(event))
                }
                _ => {
                    *state = AsyncState::Waiting(cx.waker().clone());
                    self.0.state_changed.notify_all();

                    Poll::Pending
                }
            }
        } else {
            Poll::Ready(None)
        }
    }
}

pub struct AsyncPostbox<CV, R, E>(Arc<ConnectionState<CV, AsyncState<R, E>>>)
where
    CV: Condvar;

impl<CV, R, E> AsyncPostbox<CV, R, E>
where
    CV: Condvar,
    R: Send,
    E: Send,
{
    pub fn new(connection_state: Arc<ConnectionState<CV, AsyncState<R, E>>>) -> Self {
        Self(connection_state)
    }

    pub fn post(&mut self, event: Result<Event<R>, E>) {
        let mut state = self.0.state.lock();

        loop {
            if state.is_none() {
                return;
            } else if matches!(&*state, Some(AsyncState::Received(_))) {
                state = self.0.state_changed.wait(state);
            } else {
                break;
            }
        }

        if let Some(AsyncState::Waiting(waker)) =
            mem::replace(&mut *state, Some(AsyncState::Received(event)))
        {
            waker.wake();
        }
    }
}

pub struct AsyncConnection<CV, R, E>(Arc<ConnectionState<CV, AsyncState<R, E>>>)
where
    CV: Condvar;

impl<CV, R, E> AsyncConnection<CV, R, E>
where
    CV: Condvar,
{
    pub fn new(connection_state: Arc<ConnectionState<CV, AsyncState<R, E>>>) -> Self {
        Self(connection_state)
    }
}

impl<CV, R, E> Drop for AsyncConnection<CV, R, E>
where
    CV: Condvar,
{
    fn drop(&mut self) {
        log::info!("!!!!! About to drop the MQTT async connection");

        self.0.close();

        log::info!("!!!!! The MQTT async connection dropped");
    }
}

impl<CV, R, E> Errors for AsyncConnection<CV, R, E>
where
    CV: Condvar,
    E: errors::Error,
{
    type Error = E;
}

impl<CV, R, E> Connection for AsyncConnection<CV, R, E>
where
    CV: Condvar + Send + Sync + 'static,
    <CV as MutexFamily>::Mutex<Option<AsyncState<R, E>>>: Sync + 'static,
    E: errors::Error,
{
    type Message = R;

    type NextFuture<'a>
    where
        Self: 'a,
        CV: 'a,
        R: 'a,
    = NextFuture<'a, CV, Self::Message, Self::Error>;

    fn next(&mut self) -> Self::NextFuture<'_> {
        NextFuture(&self.0)
    }
}
