use core::fmt::{Debug, Display};
use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::ops::Deref;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

extern crate alloc;
use alloc::borrow::Cow;
use alloc::sync::Arc;

use crate::mqtt::client::{Client, Enqueue, Event, Message, MessageId, Publish, QoS};
use crate::mutex::{Condvar, Mutex};
use crate::nonblocking::Unblocker;
use crate::service::Service;

pub struct EnqueueFuture<E>(Result<MessageId, E>);

impl<E> Future for EnqueueFuture<E>
where
    E: Clone,
{
    type Output = Result<MessageId, E>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0.as_ref() {
            Ok(message_id) => Poll::Ready(Ok(*message_id)),
            Err(err) => Poll::Ready(Err(err.clone())),
        }
    }
}

impl<E> crate::mqtt::client::nonblocking::Publish for E
where
    E: Enqueue,
    E::Error: Clone,
{
    type PublishFuture<'a>
    where
        Self: 'a,
    = EnqueueFuture<E::Error>;

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
        EnqueueFuture(self.enqueue(topic, qos, retain, payload))
    }
}

impl<M, P, U> Service for (Arc<M>, U)
where
    M: Mutex<Data = P>,
    P: Service,
{
    type Error = P::Error;
}

impl<M, C, U> crate::mqtt::client::nonblocking::Client for (Arc<M>, U)
where
    M: Mutex<Data = C>,
    C: Client,
    C::Error: Clone,
    U: Unblocker,
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

impl<M, P, U> crate::mqtt::client::nonblocking::Publish for (Arc<M>, U)
where
    M: Mutex<Data = P>,
    P: Publish,
    P::Error: Clone,
    U: Unblocker,
{
    type PublishFuture<'a>
    where
        Self: 'a,
    = U::UnblockFuture<Result<MessageId, P::Error>>;

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

pub struct Payload {
    event: Option<*const core::ffi::c_void>,
    waker: Option<Waker>,
    handed_over: bool,
}

unsafe impl Send for Payload {}
unsafe impl Sync for Payload {}

struct ConnectionState<CV>
where
    CV: Condvar,
{
    payload: CV::Mutex<Payload>,
    state_changed: CV,
}

pub struct EventRef<'a, M, E>(&'a Option<Result<Event<M>, E>>)
where
    M: Message + 'a,
    E: 'a;

impl<'a, M, E> Deref for EventRef<'a, M, E>
where
    M: Message + 'a,
    E: 'a,
{
    type Target = Option<Result<Event<M>, E>>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

pub struct NextFuture<'a, CV, M, E>
where
    CV: Condvar + 'a,
    M: Message + 'a,
    E: Send + 'a,
{
    connection_state: &'a ConnectionState<CV>,
    _message: PhantomData<fn() -> M>,
    _error: PhantomData<fn() -> E>,
}

impl<'a, CV, M, E> Drop for NextFuture<'a, CV, M, E>
where
    CV: Condvar + 'a,
    M: Message + 'a,
    E: Send + 'a,
{
    fn drop(&mut self) {
        let mut payload = self.connection_state.payload.lock();

        payload.waker = None;

        if payload.handed_over {
            payload.event = None;
            self.connection_state.state_changed.notify_all();
        }
    }
}

impl<'a, CV, M, E> Future for NextFuture<'a, CV, M, E>
where
    CV: Condvar + 'a,
    M: Message + 'a,
    E: Send + 'a,
{
    type Output = EventRef<'a, M, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut payload = self.connection_state.payload.lock();

        if let Some(event) = payload.event {
            payload.handed_over = true;
            Poll::Ready(EventRef(unsafe {
                (event as *const Option<Result<Event<M>, E>>)
                    .as_ref()
                    .unwrap()
            }))
        } else {
            payload.waker = Some(cx.waker().clone());
            self.connection_state.state_changed.notify_all();

            Poll::Pending
        }
    }
}

pub struct Connection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: Send,
{
    connection_state: Arc<ConnectionState<CV>>,
    _message: PhantomData<fn() -> M>,
    _error: PhantomData<fn() -> E>,
}

impl<CV, M, E> Connection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: Send,
{
    pub fn new() -> Self {
        Self {
            connection_state: Arc::new(ConnectionState {
                payload: CV::Mutex::new(Payload {
                    event: None,
                    waker: None,
                    handed_over: false,
                }),
                state_changed: CV::new(),
            }),
            _message: PhantomData,
            _error: PhantomData,
        }
    }

    pub fn post<'a>(&'a self, event: Option<Result<Event<M>, E>>)
    where
        M: 'a,
        E: 'a,
    {
        let mut payload = self.connection_state.payload.lock();

        while payload.event.is_some() {
            let waker = mem::replace(&mut payload.waker, None);
            if let Some(waker) = waker {
                waker.wake();
            }

            payload = self.connection_state.state_changed.wait(payload);
        }

        payload.event = Some(&event as *const _ as *const _);

        while payload.event.is_some() {
            let waker = mem::replace(&mut payload.waker, None);
            if let Some(waker) = waker {
                waker.wake();
            }

            payload = self.connection_state.state_changed.wait(payload);
        }
    }
}

impl<CV, M, E> Default for Connection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: Send,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<CV, M, E> Clone for Connection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: Send,
{
    fn clone(&self) -> Self {
        Self {
            connection_state: self.connection_state.clone(),
            _message: PhantomData,
            _error: PhantomData,
        }
    }
}

impl<CV, M, E> crate::service::Service for Connection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: Debug + Display + Send + Sync + 'static,
{
    type Error = E;
}

impl<CV, M, E> crate::mqtt::client::nonblocking::Connection for Connection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: Debug + Display + Send + Sync + 'static,
{
    type Message<'a>
    where
        CV: 'a,
        M: 'a,
    = M;

    type Reference<'a>
    where
        CV: 'a,
        M: Message + 'a,
        E: Send + 'a,
    = EventRef<'a, Self::Message<'a>, Self::Error>;

    type NextFuture<'a>
    where
        Self: 'a,
    = NextFuture<'a, CV, Self::Message<'a>, Self::Error>;

    fn next(&mut self) -> Self::NextFuture<'_> {
        NextFuture {
            connection_state: &self.connection_state,
            _message: PhantomData,
            _error: PhantomData,
        }
    }
}
