use core::fmt::{Debug, Display};
use core::future::Future;
use core::mem;
use core::ops::Deref;
use core::pin::Pin;
use core::task::{Poll, Waker};

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

    fn poll(self: Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
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

pub struct Payload<M, E>
where
    M: Message,
{
    event: Option<Option<Result<Event<M>, E>>>,
    waker: Option<Waker>,
}

struct ConnectionState<CV, M, E>
where
    CV: Condvar,
    M: Message + Send,
    E: Send,
{
    payload: CV::Mutex<Payload<M, E>>,
    processed: CV,
}

#[derive(Clone)]
pub struct Connection<CV, M, E>(Arc<ConnectionState<CV, M, E>>)
where
    CV: Condvar,
    M: Message + Send,
    E: Send;

pub struct NextFuture<'a, CV, M, E>(&'a ConnectionState<CV, M, E>)
where
    CV: Condvar,
    M: Message + Send,
    E: Send;

impl<'a, CV, M, E> Drop for NextFuture<'a, CV, M, E>
where
    CV: Condvar,
    M: Message + Send,
    E: Send,
{
    fn drop(&mut self) {
        let mut payload = self.0.payload.lock();

        payload.event = None;
        payload.waker = None;

        self.0.processed.notify_all();
    }
}

pub struct EventRef<'a, CV, M, E>(<<CV as Condvar>::Mutex<Payload<M, E>> as Mutex>::Guard<'a>)
where
    CV: Condvar,
    <CV as Condvar>::Mutex<Payload<M, E>>: 'a,
    M: Message + Send + 'a,
    E: Send + 'a;

impl<'a, CV, M, E> Deref for EventRef<'a, CV, M, E>
where
    CV: Condvar,
    <CV as Condvar>::Mutex<Payload<M, E>>: 'a,
    M: Message + Send + 'a,
    E: Send + 'a,
{
    type Target = Option<Result<Event<M>, E>>;

    fn deref(&self) -> &Self::Target {
        self.0.event.as_ref().unwrap()
    }
}

impl<'a, CV, M, E> Future for NextFuture<'a, CV, M, E>
where
    CV: Condvar,
    M: Message + Send,
    E: Send,
{
    type Output = EventRef<'a, CV, M, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut payload = self.0.payload.lock();

        if payload.event.is_some() {
            Poll::Ready(EventRef(payload))
        } else {
            payload.waker = Some(cx.waker().clone());

            Poll::Pending
        }
    }
}

impl<CV, M, E> Connection<CV, M, E>
where
    CV: Condvar,
    M: Message + Send,
    E: Send,
{
    pub fn new() -> Self {
        Self(Arc::new(ConnectionState {
            payload: CV::Mutex::new(Payload {
                event: None,
                waker: None,
            }),
            processed: CV::new(),
        }))
    }

    pub fn post(&self, event: Option<Result<Event<M>, E>>) {
        let mut payload = self.0.payload.lock();

        while payload.event.is_some() {
            payload = self.0.processed.wait(payload);
        }

        payload.event = Some(event);

        let waker = mem::replace(&mut payload.waker, None);

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<CV, M, E> crate::service::Service for Connection<CV, M, E>
where
    CV: Condvar,
    M: Message + Send,
    E: Debug + Display + Send + Sync + 'static,
{
    type Error = E;
}

impl<CV, M, E> crate::mqtt::client::nonblocking::Connection for Connection<CV, M, E>
where
    CV: Condvar,
    M: Message + Send,
    E: Debug + Display + Send + Sync + 'static,
{
    type Message = M;

    type Reference<'a>
    where
        CV: 'a,
        <CV as Condvar>::Mutex<Payload<M, E>>: 'a,
        M: Message + Send + 'a,
        E: Send + 'a,
    = EventRef<'a, CV, Self::Message, Self::Error>;

    type NextFuture<'a>
    where
        Self: 'a,
    = NextFuture<'a, CV, Self::Message, Self::Error>;

    fn next(&mut self) -> Self::NextFuture<'_> {
        NextFuture(&self.0)
    }
}
