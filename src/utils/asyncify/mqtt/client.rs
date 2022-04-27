use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

extern crate alloc;
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::errors::Errors;
use crate::mqtt::client::asyncs::{Client, Connection, Event, Message, MessageId, Publish, QoS};
use crate::mqtt::client::utils::{ConnectionState, State};
use crate::mutex::{Condvar, Mutex, MutexFamily};
use crate::unblocker::asyncs::Unblocker;

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

pub struct AsyncClient<U, M>(Arc<M>, U);

impl<U, M, P> AsyncClient<U, M>
where
    M: Mutex<Data = P>,
{
    pub fn new(unblocker: U, client: P) -> Self {
        Self(Arc::new(M::new(client)), unblocker)
    }
}

impl<U, M, P> Errors for AsyncClient<U, M>
where
    M: Mutex<Data = P>,
    P: Errors,
{
    type Error = P::Error;
}

impl<U, M, P> Clone for AsyncClient<U, M>
where
    U: Clone,
    M: Mutex<Data = P>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
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

        self.1
            .unblock(move || client.lock().publish(&topic, qos, retain, &payload))
    }
}

impl<U, M, P> crate::utils::asyncify::AsyncWrapper<U, P> for AsyncClient<U, M>
where
    M: Mutex<Data = P>,
{
    fn new(unblocker: U, sync: P) -> Self {
        AsyncClient::new(unblocker, sync)
    }
}

pub struct ConnectionStateAsyncPayload {
    event: Option<*const core::ffi::c_void>,
    waker: Option<Waker>,
    handed_over: bool,
}

impl ConnectionStateAsyncPayload {
    pub fn new() -> Self {
        Self {
            event: None,
            waker: None,
            handed_over: false,
        }
    }
}

impl Default for ConnectionStateAsyncPayload {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for ConnectionStateAsyncPayload {}
unsafe impl Sync for ConnectionStateAsyncPayload {}

pub struct NextFuture<'a, CV, F, O, M, E>
where
    CV: Condvar + 'a,
    M: Message + 'a,
    E: 'a,
{
    connection_state: &'a ConnectionState<CV, ConnectionStateAsyncPayload>,
    converter: Option<F>,
    _output: PhantomData<fn() -> O>,
    _message: PhantomData<fn() -> M>,
    _error: PhantomData<fn() -> E>,
}

impl<'a, CV, F, O, M, E> Drop for NextFuture<'a, CV, F, O, M, E>
where
    CV: Condvar + 'a,
    M: Message + 'a,
    E: 'a,
{
    fn drop(&mut self) {
        let mut state = self.connection_state.state.lock();

        state.payload.waker = None;

        if state.payload.handed_over {
            state.payload.event = None;
            self.connection_state.state_changed.notify_all();
        }
    }
}

impl<'a, CV, F, O, M, E> Future for NextFuture<'a, CV, F, O, M, E>
where
    CV: Condvar + 'a,
    F: FnOnce(&Result<Event<M>, E>) -> O + Unpin,
    M: Message + 'a,
    E: 'a,
{
    type Output = Option<O>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.connection_state.state.lock();

        if state.closed {
            Poll::Ready(None)
        } else if let Some(event) = state.payload.event {
            let converter = mem::replace(&mut self.converter, None).unwrap();

            let event_ref = unsafe {
                (event as *const Option<Result<Event<M>, E>>)
                    .as_ref()
                    .unwrap()
            };

            let result = event_ref.as_ref().map(|result| converter(result));
            state.payload.handed_over = true;

            Poll::Ready(result)
        } else {
            state.payload.waker = Some(cx.waker().clone());
            self.connection_state.state_changed.notify_all();

            Poll::Pending
        }
    }
}

pub struct AsyncPoster<CV, M, E>
where
    CV: Condvar,
    M: Message,
{
    connection_state: Arc<ConnectionState<CV, ConnectionStateAsyncPayload>>,
    _message: PhantomData<fn() -> M>,
    _error: PhantomData<fn() -> E>,
}

impl<CV, M, E> AsyncPoster<CV, M, E>
where
    CV: Condvar,
    M: Message,
{
    pub fn new(connection_state: Arc<ConnectionState<CV, ConnectionStateAsyncPayload>>) -> Self {
        Self {
            connection_state,
            _message: PhantomData,
            _error: PhantomData,
        }
    }

    pub fn post<'a>(&'a self, event: &'a Option<Result<Event<M>, E>>)
    where
        M: 'a,
        E: 'a,
    {
        let mut state = self.connection_state.state.lock();

        while !state.closed && state.payload.event.is_some() {
            let waker = mem::replace(&mut state.payload.waker, None);
            if let Some(waker) = waker {
                waker.wake();
            }

            state = self.connection_state.state_changed.wait(state);
        }

        state.payload.event = Some(event as *const _ as *const _);

        while !state.closed && state.payload.event.is_some() {
            let waker = mem::replace(&mut state.payload.waker, None);
            if let Some(waker) = waker {
                waker.wake();
            }

            state = self.connection_state.state_changed.wait(state);
        }
    }
}

pub struct AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
{
    connection_state: Arc<ConnectionState<CV, ConnectionStateAsyncPayload>>,
    _message: PhantomData<fn() -> M>,
    _error: PhantomData<fn() -> E>,
}

impl<CV, M, E> AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
{
    pub fn new(connection_state: Arc<ConnectionState<CV, ConnectionStateAsyncPayload>>) -> Self {
        Self {
            connection_state,
            _message: PhantomData,
            _error: PhantomData,
        }
    }
}

impl<CV, M, E> Drop for AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
{
    fn drop(&mut self) {
        self.connection_state.close();
    }
}

#[cfg(not(feature = "std"))]
impl<CV, M, E> Errors for AsyncConnection<CV, M, E>
where
    CV: Condvar,
    M: Message,
    E: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static,
{
    type Error = E;
}

#[cfg(feature = "std")]
impl<CV, M, E> Errors for AsyncConnection<CV, M, E>
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
    CV: Condvar + Send + Sync + 'static,
    <CV as MutexFamily>::Mutex<State<ConnectionStateAsyncPayload>>: Sync + 'static,
    M: Message,
    E: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static,
{
    type Message = M;

    type NextFuture<'a, F, O>
    where
        Self: 'a,
        CV: 'a,
        M: 'a,
        F: FnOnce(&Result<Event<Self::Message>, Self::Error>) -> O + Unpin + Send,
    = NextFuture<'a, CV, F, O, Self::Message, Self::Error>;

    fn next<'a, F, O>(&'a mut self, f: F) -> Self::NextFuture<'a, F, O>
    where
        F: FnOnce(&Result<Event<Self::Message>, Self::Error>) -> O + Unpin + Send,
    {
        NextFuture {
            connection_state: &self.connection_state,
            converter: Some(f),
            _output: PhantomData,
            _message: PhantomData,
            _error: PhantomData,
        }
    }
}

#[cfg(feature = "std")]
impl<CV, M, E> Connection for AsyncConnection<CV, M, E>
where
    CV: Condvar + Send + Sync + 'static,
    <CV as MutexFamily>::Mutex<State<ConnectionStateAsyncPayload>>: Sync + 'static,
    M: Message,
    E: std::error::Error + Send + Sync + 'static,
{
    type Message = M;

    type NextFuture<'a, F, O>
    where
        Self: 'a,
        CV: 'a,
        M: 'a,
        F: FnOnce(&Result<Event<Self::Message>, Self::Error>) -> O + Unpin + Send,
    = NextFuture<'a, CV, F, O, Self::Message, Self::Error>;

    fn next<'a, F, O>(&'a mut self, f: F) -> Self::NextFuture<'a, F, O>
    where
        F: FnOnce(&Result<Event<Self::Message>, Self::Error>) -> O + Unpin + Send,
    {
        NextFuture {
            connection_state: &self.connection_state,
            converter: Some(f),
            _output: PhantomData,
            _message: PhantomData,
            _error: PhantomData,
        }
    }
}
