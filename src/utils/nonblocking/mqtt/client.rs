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

use crate::mqtt::client::{Enqueue, Event, Message, MessageId, QoS};
use crate::mutex::{Condvar, Mutex};

pub struct PublishFuture<E>(Result<MessageId, E>);

impl<E> Future for PublishFuture<E>
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
    type PublishFuture = PublishFuture<E::Error>;

    fn publish<'a, S, V>(
        &'a mut self,
        topic: S,
        qos: QoS,
        retain: bool,
        payload: V,
    ) -> Self::PublishFuture
    where
        S: Into<Cow<'a, str>>,
        V: Into<Cow<'a, [u8]>>,
    {
        PublishFuture(self.enqueue(topic, qos, retain, payload))
    }
}

pub struct Payload {
    event: Option<*const core::ffi::c_void>,
    waker: Option<Waker>,
    processed: bool,
}

unsafe impl Send for Payload {}
unsafe impl Sync for Payload {}

struct ConnectionState<CV>
where
    CV: Condvar,
{
    payload: CV::Mutex<Payload>,
    processed: CV,
}

pub struct EventRef<'a, CV, M, E>
where
    CV: Condvar,
    <CV as Condvar>::Mutex<Payload>: 'a,
    M: Message + 'a,
    E: 'a,
{
    payload: <<CV as Condvar>::Mutex<Payload> as Mutex>::Guard<'a>,
    _message: PhantomData<fn() -> M>,
    _error: PhantomData<fn() -> E>,
}

impl<'a, CV, M, E> Deref for EventRef<'a, CV, M, E>
where
    CV: Condvar,
    <CV as Condvar>::Mutex<Payload>: 'a,
    M: Message + 'a,
    E: 'a,
{
    type Target = Option<Result<Event<M>, E>>;

    fn deref(&self) -> &Self::Target {
        let event = self.payload.event.unwrap() as *const Self::Target;

        unsafe { event.as_ref().unwrap() }
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

        if payload.processed {
            payload.event = None;
            self.connection_state.processed.notify_all();
        }
    }
}

impl<'a, CV, M, E> Future for NextFuture<'a, CV, M, E>
where
    CV: Condvar + 'a,
    M: Message + 'a,
    E: Send + 'a,
{
    type Output = EventRef<'a, CV, M, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut payload = self.connection_state.payload.lock();

        if payload.event.is_some() {
            payload.processed = true;
            Poll::Ready(EventRef {
                payload,
                _message: PhantomData,
                _error: PhantomData,
            })
        } else {
            payload.waker = Some(cx.waker().clone());

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
                    processed: false,
                }),
                processed: CV::new(),
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
            payload = self.connection_state.processed.wait(payload);
        }

        payload.event = Some(&event as *const _ as *const _);

        let waker = mem::replace(&mut payload.waker, None);

        if let Some(waker) = waker {
            waker.wake();
        }

        while payload.event.is_some() {
            payload = self.connection_state.processed.wait(payload);
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
        <CV as Condvar>::Mutex<Payload>: 'a,
        M: Message + 'a,
        E: Send + 'a,
    = EventRef<'a, CV, Self::Message<'a>, Self::Error>;

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
