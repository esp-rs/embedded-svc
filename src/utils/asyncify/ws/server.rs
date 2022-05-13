use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use core::{mem, slice};

extern crate alloc;
use alloc::borrow::ToOwned;
use alloc::sync::Arc;
use alloc::vec::Vec;

use heapless;

use log::info;

use crate::errors::*;
use crate::mutex::*;
use crate::unblocker::asyncs::Unblocker;
use crate::ws::{server::*, *};

pub struct AsyncSender<U, S> {
    unblocker: U,
    sender: S,
}

impl<U, S> Errors for AsyncSender<U, S>
where
    S: Errors,
{
    type Error = S::Error;
}

impl<U, S> asyncs::Sender for AsyncSender<U, S>
where
    U: Unblocker,
    S: Sender + SessionProvider + Send + Clone + 'static,
{
    type SendFuture<'a>
    where
        Self: 'a,
    = U::UnblockFuture<Result<(), Self::Error>>;

    fn send(&mut self, frame_type: FrameType, frame_data: Option<&[u8]>) -> Self::SendFuture<'_> {
        info!(
            "Sending data (frame_type={:?}, frame_len={}) to WS connection {:?}",
            frame_type,
            frame_data.map(|d| d.len()).unwrap_or(0),
            self.sender.session()
        );

        let mut sender = self.sender.clone();
        let frame_data: Option<Vec<u8>> = frame_data.map(|frame_data| frame_data.to_owned());

        self.unblocker
            .unblock(move || sender.send(frame_type, frame_data.as_deref()))
    }
}

impl<S> asyncs::Sender for AsyncSender<(), S>
where
    S: Sender + SessionProvider + Send + Clone + 'static,
{
    type SendFuture<'a>
    where
        Self: 'a,
    = impl Future<Output = Result<(), Self::Error>>;

    fn send(&mut self, frame_type: FrameType, frame_data: Option<&[u8]>) -> Self::SendFuture<'_> {
        info!(
            "Sending data (frame_type={:?}, frame_len={}) to WS connection {:?}",
            frame_type,
            frame_data.map(|d| d.len()).unwrap_or(0),
            self.sender.session()
        );

        let mut sender = self.sender.clone();
        let frame_data: Option<Vec<u8>> = frame_data.map(|frame_data| frame_data.to_owned());

        async move { sender.send(frame_type, frame_data.as_deref()) }
    }
}

pub enum ReceiverData {
    None,
    Metadata((FrameType, usize)),
    Data(*mut u8),
    DataCopied,
    Closed,
}

unsafe impl Send for ReceiverData {}

pub struct SharedReceiverState {
    waker: Option<Waker>,
    data: ReceiverData,
}

pub struct ConnectionState<M, S> {
    session: S,
    receiver_state: Arc<M>,
}

pub struct AsyncReceiverFuture<'a, C, E>
where
    C: Condvar,
{
    receiver: &'a mut AsyncReceiver<C, E>,
    frame_data_buf: &'a mut [u8],
}

impl<'a, C, E> Future for AsyncReceiverFuture<'a, C, E>
where
    C: Condvar,
{
    type Output = Result<(FrameType, usize), E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let frame_data_buf_ptr = self.frame_data_buf.as_mut_ptr();
        let mut shared = self.receiver.shared.lock();

        if let ReceiverData::Metadata((frame_type, size)) = shared.data {
            if self.frame_data_buf.len() >= size {
                shared.data = ReceiverData::Data(frame_data_buf_ptr);

                self.receiver.condvar.notify_all();

                while !matches!(shared.data, ReceiverData::DataCopied) {
                    shared = self.receiver.condvar.wait(shared);
                }

                shared.data = ReceiverData::None;
                self.receiver.condvar.notify_all();
            }

            Poll::Ready(Ok((frame_type, size)))
        } else if let ReceiverData::Closed = shared.data {
            Poll::Ready(Ok((FrameType::Close, 0)))
        } else {
            shared.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct AsyncReceiver<C, E>
where
    C: Condvar,
{
    _error: PhantomData<fn() -> E>,
    shared: Arc<C::Mutex<SharedReceiverState>>,
    condvar: Arc<C>,
}

impl<C, E> Errors for AsyncReceiver<C, E>
where
    C: Condvar,
    E: Error,
{
    type Error = E;
}

impl<C, E> asyncs::Receiver for AsyncReceiver<C, E>
where
    C: Condvar + Send + Sync,
    <C as MutexFamily>::Mutex<SharedReceiverState>: Send + Sync,
    E: Error,
{
    type ReceiveFuture<'a>
    where
        Self: 'a,
    = AsyncReceiverFuture<'a, C, E>;

    fn recv<'a>(&'a mut self, frame_data_buf: &'a mut [u8]) -> Self::ReceiveFuture<'a> {
        AsyncReceiverFuture {
            receiver: self,
            frame_data_buf,
        }
    }
}

#[allow(clippy::type_complexity)]
pub struct SharedAcceptorState<C, S>
where
    C: Condvar + Send + Sync,
    C::Mutex<SharedReceiverState>: Send + Sync,
    S: Send,
{
    waker: Option<Waker>,
    data: Option<Option<(Arc<C::Mutex<SharedReceiverState>>, S)>>,
}

pub struct AsyncAcceptor<U, C, S>
where
    C: Condvar + Send + Sync,
    C::Mutex<SharedReceiverState>: Send + Sync,
    C::Mutex<SharedAcceptorState<C, S>>: Send + Sync,
    S: Send,
{
    unblocker: U,
    accept: Arc<C::Mutex<SharedAcceptorState<C, S>>>,
    condvar: Arc<C>,
}

impl<U, C, S> Errors for AsyncAcceptor<U, C, S>
where
    C: Condvar + Send + Sync,
    C::Mutex<SharedReceiverState>: Send + Sync,
    C::Mutex<SharedAcceptorState<C, S>>: Send + Sync,
    S: Send + Errors,
{
    type Error = <S as Errors>::Error;
}

impl<'a, U, C, S> Future for &'a mut AsyncAcceptor<U, C, S>
where
    U: Clone,
    C: Condvar + Send + Sync,
    C::Mutex<SharedReceiverState>: Send + Sync,
    C::Mutex<SharedAcceptorState<C, S>>: Send + Sync,
    S: Sender + Errors + Send + Clone + 'static,
{
    type Output = Result<
        Option<(AsyncSender<U, S>, AsyncReceiver<C, <S as Errors>::Error>)>,
        <S as Errors>::Error,
    >;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut accept = self.accept.lock();

        match mem::replace(&mut accept.data, None) {
            Some(Some((shared, sender))) => {
                let sender = AsyncSender {
                    unblocker: self.unblocker.clone(),
                    sender,
                };

                let receiver = AsyncReceiver {
                    _error: PhantomData,
                    shared,
                    condvar: self.condvar.clone(),
                };

                self.condvar.notify_all();

                Poll::Ready(Ok(Some((sender, receiver))))
            }
            Some(None) => {
                accept.data = Some(None);
                Poll::Ready(Ok(None))
            }
            None => {
                accept.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl<U, C, S> asyncs::Acceptor for AsyncAcceptor<U, C, S>
where
    U: Unblocker + Clone + Send,
    C: Condvar + Send + Sync,
    C::Mutex<SharedReceiverState>: Send + Sync,
    C::Mutex<SharedAcceptorState<C, S>>: Send + Sync,
    S: Sender + SessionProvider + Errors + Send + Clone + 'static,
{
    type Sender = AsyncSender<U, S>;

    type Receiver = AsyncReceiver<C, <S as Errors>::Error>;

    type AcceptFuture<'a>
    where
        Self: 'a,
    = &'a mut Self;

    fn accept(&mut self) -> Self::AcceptFuture<'_> {
        self
    }
}

impl<C, S> asyncs::Acceptor for AsyncAcceptor<(), C, S>
where
    C: Condvar + Send + Sync,
    C::Mutex<SharedReceiverState>: Send + Sync,
    C::Mutex<SharedAcceptorState<C, S>>: Send + Sync,
    S: Sender + SessionProvider + Errors + Send + Clone + 'static,
{
    type Sender = AsyncSender<(), S>;

    type Receiver = AsyncReceiver<C, <S as Errors>::Error>;

    type AcceptFuture<'a>
    where
        Self: 'a,
    = &'a mut Self;

    fn accept(&mut self) -> Self::AcceptFuture<'_> {
        self
    }
}

pub struct Processor<C, S, R, const N: usize, const F: usize>
where
    C: Condvar + Send + Sync,
    C::Mutex<SharedReceiverState>: Send + Sync,
    C::Mutex<SharedAcceptorState<C, S::Sender>>: Send + Sync,
    S: SenderFactory,
    S::Sender: Send,
    R: SessionProvider,
{
    connections: heapless::Vec<ConnectionState<C::Mutex<SharedReceiverState>, R::Session>, N>,
    frame_data_buf: [u8; F],
    accept: Arc<C::Mutex<SharedAcceptorState<C, S::Sender>>>,
    condvar: Arc<C>,
}

impl<C, S, R, const N: usize, const F: usize> Processor<C, S, R, N, F>
where
    C: Condvar + Send + Sync,
    C::Mutex<SharedReceiverState>: Send + Sync,
    C::Mutex<SharedAcceptorState<C, S::Sender>>: Send + Sync,
    S: SenderFactory,
    S::Sender: Send,
    R: SessionProvider,
{
    pub fn new<U>(unblocker: U) -> (Self, AsyncAcceptor<U, C, S::Sender>) {
        let this = Self {
            connections: heapless::Vec::new(),
            frame_data_buf: [0_u8; F],
            accept: Arc::new(C::Mutex::new(SharedAcceptorState {
                waker: None,
                data: None,
            })),
            condvar: Arc::new(C::new()),
        };

        let acceptor = AsyncAcceptor {
            unblocker,
            accept: this.accept.clone(),
            condvar: this.condvar.clone(),
        };

        (this, acceptor)
    }

    pub fn process<'a>(&'a mut self, receiver: &'a mut R, sender: &'a mut S) -> Result<(), R::Error>
    where
        R: Receiver,
        S: Sender<Error = R::Error>,
    {
        if receiver.is_new() {
            let session = receiver.session();

            info!("New WS connection {:?}", session);

            if !self.process_accept(session, sender) {
                return sender.send(FrameType::Close, None)
            }
        } else if receiver.is_closed() {
            let session = receiver.session();

            if let Some(index) = self.connections
                .iter()
                .enumerate()
                .find_map(|(index, conn)| (conn.session == session).then(|| index))
            {
                let conn = self.connections.swap_remove(index);

                Self::process_receive_close(&conn.receiver_state);
                info!("Closed WS connection {:?}", session);
            }
        } else {
            let session = receiver.session();
            let (frame_type, len) = receiver.recv(&mut self.frame_data_buf)?;

            info!(
                "Incoming data (frame_type={:?}, frame_len={}) from WS connection {:?}",
                frame_type, len, session
            );

            self.connections
                .iter()
                .find(|receiver| receiver.session == session)
                .map(|receiver| self.process_receive(&receiver.receiver_state, frame_type, len));
        }

        Ok(())
    }

    fn process_accept<'a>(&'a mut self, session: R::Session, sender: &'a mut S) -> bool {
        if self.connections.len() < F {
            let receiver_state = Arc::new(C::Mutex::new(SharedReceiverState {
                waker: None,
                data: ReceiverData::None,
            }));

            let state = ConnectionState {
                session,
                receiver_state: receiver_state.clone(),
            };

            self.connections.push(state).unwrap_or_else(|_| panic!());

            let sender = sender.create().unwrap();

            let mut accept = self.accept.lock();

            accept.data = Some(Some((receiver_state, sender)));

            if let Some(waker) = mem::replace(&mut accept.waker, None) {
                waker.wake();
            }

            while accept.data.is_some() {
                accept = self.condvar.wait(accept);
            }

            true
        } else {
            false
        }
    }

    fn process_receive(
        &self,
        state: &C::Mutex<SharedReceiverState>,
        frame_type: FrameType,
        len: usize,
    ) {
        let mut shared = state.lock();

        shared.data = ReceiverData::Metadata((frame_type, len));

        if let Some(waker) = mem::replace(&mut shared.waker, None) {
            waker.wake();
        }

        loop {
            if let ReceiverData::Data(buf) = &shared.data {
                unsafe { slice::from_raw_parts_mut(*buf, len) }
                    .copy_from_slice(&self.frame_data_buf[..len]);
                shared.data = ReceiverData::DataCopied;
                self.condvar.notify_all();

                break;
            }

            shared = self.condvar.wait(shared);
        }

        while !matches!(shared.data, ReceiverData::None) {
            shared = self.condvar.wait(shared);
        }
    }

    fn process_accept_close(&mut self) {
        let mut accept = self.accept.lock();

        accept.data = Some(None);

        if let Some(waker) = mem::replace(&mut accept.waker, None) {
            waker.wake();
        }
    }

    fn process_receive_close(state: &C::Mutex<SharedReceiverState>) {
        let mut shared = state.lock();

        shared.data = ReceiverData::Closed;

        if let Some(waker) = mem::replace(&mut shared.waker, None) {
            waker.wake();
        }
    }
}

impl<C, S, R, const N: usize, const F: usize> Drop for Processor<C, S, R, N, F>
where
    C: Condvar + Send + Sync,
    C::Mutex<SharedReceiverState>: Send + Sync,
    C::Mutex<SharedAcceptorState<C, S::Sender>>: Send + Sync,
    S: SenderFactory,
    S::Sender: Send,
    R: SessionProvider,
{
    fn drop(&mut self) {
        self.process_accept_close();
    }
}
