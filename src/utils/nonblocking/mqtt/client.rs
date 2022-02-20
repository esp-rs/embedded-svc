use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

extern crate alloc;
use alloc::borrow::Cow;
use alloc::sync::Arc;

use crate::mqtt::client::nonblocking::{
    Client, Connection, Event, Message, MessageId, Publish, QoS,
};
use crate::mutex::{Condvar, Mutex};
use crate::service::Service;
use crate::unblocker::nonblocking::Unblocker;

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

impl<E> Publish for E
where
    E: crate::mqtt::client::Enqueue,
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

pub struct AsyncClient<U, M>(Arc<M>, PhantomData<fn() -> U>);

impl<U, M, P> AsyncClient<U, M>
where
    M: Mutex<Data = P>,
{
    pub fn new(blocking_client: P) -> Self {
        Self(Arc::new(M::new(blocking_client)), PhantomData)
    }
}

impl<U, M, P> Service for AsyncClient<U, M>
where
    M: Mutex<Data = P>,
    P: Service,
{
    type Error = P::Error;
}

impl<U, M, P> Clone for AsyncClient<U, M>
where
    M: Mutex<Data = P>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<U, M, C> Client for AsyncClient<U, M>
where
    M: Mutex<Data = C> + Send + Sync + 'static,
    C: crate::mqtt::client::Client,
    C::Error: Clone,
    U: Unblocker,
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

        U::unblock(Box::new(move || client.lock().subscribe(&topic, qos)))
    }

    fn unsubscribe<'a, S>(&'a mut self, topic: S) -> Self::UnsubscribeFuture<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        let topic: String = topic.into().into_owned();
        let client = self.0.clone();

        U::unblock(Box::new(move || client.lock().unsubscribe(&topic)))
    }
}

impl<U, M, P> Publish for AsyncClient<U, M>
where
    M: Mutex<Data = P> + Send + Sync + 'static,
    P: crate::mqtt::client::Publish,
    P::Error: Clone,
    U: Unblocker,
    Self::Error: Send + Sync + 'static,
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

        U::unblock(Box::new(move || {
            client.lock().publish(&topic, qos, retain, &payload)
        }))
    }
}

impl<U, M, P> crate::utils::nonblocking::AsyncWrapper<U, P> for AsyncClient<U, M>
where
    M: Mutex<Data = P>,
{
    fn new(sync: P) -> Self {
        AsyncClient::new(sync)
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

pub struct NextFuture<'a, CV, FM, OM, FE, OE, M, E>
where
    CV: Condvar + 'a,
    M: Message + 'a,
    E: 'a,
{
    connection_state: &'a ConnectionState<CV>,
    message_converter: FM,
    error_converter: FE,
    _output: PhantomData<fn() -> OM>,
    _error_output: PhantomData<fn() -> OE>,
    _message: PhantomData<fn() -> M>,
    _error: PhantomData<fn() -> E>,
}

impl<'a, CV, FM, OM, FE, OE, M, E> Drop for NextFuture<'a, CV, FM, OM, FE, OE, M, E>
where
    CV: Condvar + 'a,
    M: Message + 'a,
    E: 'a,
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

impl<'a, CV, FM, OM, FE, OE, M, E> Future for NextFuture<'a, CV, FM, OM, FE, OE, M, E>
where
    CV: Condvar + 'a,
    FM: FnMut(&'a M) -> OM + Unpin,
    FE: FnMut(&'a E) -> OE + Unpin,
    M: Message + 'a,
    E: 'a,
{
    type Output = Option<Result<Event<OM>, OE>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut payload = self.connection_state.payload.lock();

        if let Some(event) = payload.event {
            let event_ref = unsafe {
                (event as *const Option<Result<Event<M>, E>>)
                    .as_ref()
                    .unwrap()
            };

            let result = match event_ref {
                Some(Ok(event)) => match event {
                    Event::Received(message) => {
                        Some(Ok(Event::Received((self.message_converter)(message))))
                    }
                    Event::BeforeConnect => Some(Ok(Event::BeforeConnect)),
                    Event::Connected(session) => Some(Ok(Event::Connected(*session))),
                    Event::Disconnected => Some(Ok(Event::Disconnected)),
                    Event::Subscribed(message_id) => Some(Ok(Event::Subscribed(*message_id))),
                    Event::Unsubscribed(message_id) => Some(Ok(Event::Unsubscribed(*message_id))),
                    Event::Published(message_id) => Some(Ok(Event::Published(*message_id))),
                    Event::Deleted(message_id) => Some(Ok(Event::Deleted(*message_id))),
                },
                Some(Err(error)) => Some(Err((self.error_converter)(error))),
                None => None,
            };

            payload.handed_over = true;

            Poll::Ready(result)
        } else {
            payload.waker = Some(cx.waker().clone());
            self.connection_state.state_changed.notify_all();

            Poll::Pending
        }
    }
}

pub struct AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
{
    connection_state: Arc<ConnectionState<CV>>,
    _message: PhantomData<fn() -> M>,
    _error: PhantomData<fn() -> E>,
}

impl<CV, M, E> AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
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

    pub fn post<'a>(&'a self, event: &'a Option<Result<Event<M>, E>>)
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

        payload.event = Some(event as *const _ as *const _);

        while payload.event.is_some() {
            let waker = mem::replace(&mut payload.waker, None);
            if let Some(waker) = waker {
                waker.wake();
            }

            payload = self.connection_state.state_changed.wait(payload);
        }
    }
}

impl<CV, M, E> Default for AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<CV, M, E> Clone for AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
{
    fn clone(&self) -> Self {
        Self {
            connection_state: self.connection_state.clone(),
            _message: PhantomData,
            _error: PhantomData,
        }
    }
}

#[cfg(not(feature = "std"))]
impl<CV, M, E> Service for AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static,
{
    type Error = E;
}

#[cfg(feature = "std")]
impl<CV, M, E> Service for AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: std::error::Error + Send + Sync + 'static,
{
    type Error = E;
}

#[cfg(not(feature = "std"))]
impl<CV, M, E> Connection for AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static,
{
    type Message<'a>
    where
        CV: 'a,
        M: 'a,
    = M;

    type NextFuture<'a, FM, OM, FE, OE>
    where
        Self: 'a,
        CV: 'a,
        M: 'a,
        FM: FnMut(&'a Self::Message<'a>) -> OM + Unpin,
        FE: FnMut(&'a Self::Error) -> OE + Unpin,
    = NextFuture<'a, CV, FM, OM, FE, OE, Self::Message<'a>, Self::Error>;

    fn next<'a, FM, OM, FE, OE>(
        &'a mut self,
        fm: FM,
        fe: FE,
    ) -> Self::NextFuture<'a, FM, OM, FE, OE>
    where
        FM: FnMut(&'a Self::Message<'a>) -> OM + Unpin,
        FE: FnMut(&'a Self::Error) -> OE + Unpin,
    {
        NextFuture {
            connection_state: &self.connection_state,
            message_converter: fm,
            error_converter: fe,
            _output: PhantomData,
            _error_output: PhantomData,
            _message: PhantomData,
            _error: PhantomData,
        }
    }
}

#[cfg(feature = "std")]
impl<CV, M, E> Connection for AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: std::error::Error + Send + Sync + 'static,
{
    type Message<'a>
    where
        CV: 'a,
        M: 'a,
    = M;

    type NextFuture<'a, FM, OM, FE, OE>
    where
        Self: 'a,
        CV: 'a,
        M: 'a,
        FM: FnMut(&'a Self::Message<'a>) -> OM + Unpin,
        FE: FnMut(&'a Self::Error) -> OE + Unpin,
    = NextFuture<'a, CV, FM, OM, FE, OE, Self::Message<'a>, Self::Error>;

    fn next<'a, FM, OM, FE, OE>(
        &'a mut self,
        fm: FM,
        fe: FE,
    ) -> Self::NextFuture<'a, FM, OM, FE, OE>
    where
        FM: FnMut(&'a Self::Message<'a>) -> OM + Unpin,
        FE: FnMut(&'a Self::Error) -> OE + Unpin,
    {
        NextFuture {
            connection_state: &self.connection_state,
            message_converter: fm,
            error_converter: fe,
            _output: PhantomData,
            _error_output: PhantomData,
            _message: PhantomData,
            _error: PhantomData,
        }
    }
}
