#[cfg(feature = "alloc")]
pub mod server {
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll, Waker};
    use core::{mem, slice};

    extern crate alloc;
    use alloc::borrow::ToOwned;
    use alloc::sync::Arc;
    use alloc::vec::Vec;

    use heapless;

    use log::info;

    use crate::mutex::RawCondvar;
    use crate::utils::mutex::{Condvar, Mutex};
    use crate::ws::{callback_server::*, *};

    #[cfg(all(feature = "nightly", feature = "experimental"))]
    pub use async_traits_impl::*;

    #[allow(dead_code)]
    pub struct AsyncConnection<U, C, S>
    where
        C: RawCondvar,
    {
        unblocker: U,
        sender: S,
        shared: Arc<Mutex<C::RawMutex, SharedReceiverState>>,
        condvar: Arc<Condvar<C>>,
    }

    impl<C, S> AsyncConnection<(), C, S>
    where
        C: RawCondvar,
        S: Sender + SessionProvider + Send + Clone + 'static,
    {
        pub async fn send(
            &mut self,
            frame_type: FrameType,
            frame_data: &[u8],
        ) -> Result<(), S::Error> {
            info!(
                "Sending data (frame_type={:?}, frame_len={}) to WS connection {:?}",
                frame_type,
                frame_data.len(),
                self.sender.session()
            );

            let mut sender = self.sender.clone();
            let frame_data: Vec<u8> = frame_data.to_owned();

            async move { sender.send(frame_type, &frame_data) }.await
        }
    }

    impl<U, C, S> AsyncConnection<U, C, S>
    where
        U: Send,
        C: RawCondvar + Send + Sync,
        C::RawMutex: Send + Sync,
        S: ErrorType + Send,
    {
        pub fn recv<'a>(
            &'a mut self,
            frame_data_buf: &'a mut [u8],
        ) -> AsyncReceiverFuture<'a, U, C, S> {
            AsyncReceiverFuture {
                receiver: self,
                frame_data_buf,
            }
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

    pub struct AsyncReceiverFuture<'a, U, C, S>
    where
        C: RawCondvar,
    {
        receiver: &'a mut AsyncConnection<U, C, S>,
        frame_data_buf: &'a mut [u8],
    }

    impl<'a, U, C, S> Future for AsyncReceiverFuture<'a, U, C, S>
    where
        C: RawCondvar,
        S: ErrorType,
    {
        type Output = Result<(FrameType, usize), S::Error>;

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

    #[allow(clippy::type_complexity)]
    pub struct SharedAcceptorState<C, S>
    where
        C: RawCondvar + Send + Sync,
        C::RawMutex: Send + Sync,
        S: Send,
    {
        waker: Option<Waker>,
        data: Option<Option<(Arc<Mutex<C::RawMutex, SharedReceiverState>>, S)>>,
    }

    pub struct AsyncAcceptor<U, C, S>
    where
        C: RawCondvar + Send + Sync,
        C::RawMutex: Send + Sync,
        S: Send,
    {
        unblocker: U,
        accept: Arc<Mutex<C::RawMutex, SharedAcceptorState<C, S>>>,
        condvar: Arc<Condvar<C>>,
    }

    impl<U, C, S> AsyncAcceptor<U, C, S>
    where
        C: RawCondvar + Send + Sync,
        C::RawMutex: Send + Sync,
        S: Sender + SessionProvider + Send + Clone + 'static,
        S::Error: Send + Sync + 'static,
    {
        pub fn accept(&self) -> &AsyncAcceptor<U, C, S> {
            self
        }
    }

    impl<'a, U, C, S> Future for &'a AsyncAcceptor<U, C, S>
    where
        U: Clone,
        C: RawCondvar + Send + Sync,
        C::RawMutex: Send + Sync,
        S: Sender + Send + Clone + 'static,
    {
        type Output = Result<AsyncConnection<U, C, S>, <S as ErrorType>::Error>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut accept = self.accept.lock();

            match mem::replace(&mut accept.data, None) {
                Some(Some((shared, sender))) => {
                    let connection = AsyncConnection {
                        unblocker: self.unblocker.clone(),
                        sender,
                        shared,
                        condvar: self.condvar.clone(),
                    };

                    self.condvar.notify_all();

                    Poll::Ready(Ok(connection))
                }
                Some(None) => {
                    accept.data = Some(None);
                    Poll::Pending
                }
                None => {
                    accept.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }

    #[allow(clippy::type_complexity)]
    pub struct Processor<const N: usize, const F: usize, C, W>
    where
        C: RawCondvar + Send + Sync,
        C::RawMutex: Send + Sync,
        W: SenderFactory + SessionProvider,
        W::Sender: Send,
    {
        connections:
            heapless::Vec<ConnectionState<Mutex<C::RawMutex, SharedReceiverState>, W::Session>, N>,
        frame_data_buf: [u8; F],
        accept: Arc<Mutex<C::RawMutex, SharedAcceptorState<C, W::Sender>>>,
        condvar: Arc<Condvar<C>>,
    }

    impl<const N: usize, const F: usize, C, W> Processor<N, F, C, W>
    where
        C: RawCondvar + Send + Sync,
        C::RawMutex: Send + Sync,
        W: SenderFactory + SessionProvider,
        W::Sender: Send,
    {
        pub fn new<U>(unblocker: U) -> (Self, AsyncAcceptor<U, C, W::Sender>) {
            let this = Self {
                connections: heapless::Vec::new(),
                frame_data_buf: [0_u8; F],
                accept: Arc::new(Mutex::new(SharedAcceptorState {
                    waker: None,
                    data: None,
                })),
                condvar: Arc::new(Condvar::new()),
            };

            let acceptor = AsyncAcceptor {
                unblocker,
                accept: this.accept.clone(),
                condvar: this.condvar.clone(),
            };

            (this, acceptor)
        }

        pub fn process<'a>(&'a mut self, connection: &'a mut W) -> Result<(), W::Error>
        where
            W: Sender + Receiver,
        {
            if connection.is_new() {
                let session = connection.session();

                info!("New WS connection {:?}", session);

                if !self.process_accept(session, connection) {
                    return connection.send(FrameType::Close, &[]);
                }
            } else if connection.is_closed() {
                let session = connection.session();

                if let Some(index) = self
                    .connections
                    .iter()
                    .enumerate()
                    .find_map(|(index, conn)| (conn.session == session).then(|| index))
                {
                    let conn = self.connections.swap_remove(index);

                    Self::process_receive_close(&conn.receiver_state);
                    info!("Closed WS connection {:?}", session);
                }
            } else {
                let session = connection.session();
                let (frame_type, len) = connection.recv(&mut self.frame_data_buf)?;

                info!(
                    "Incoming data (frame_type={:?}, frame_len={}) from WS connection {:?}",
                    frame_type, len, session
                );

                if let Some(connection) = self
                    .connections
                    .iter()
                    .find(|connection| connection.session == session)
                {
                    self.process_receive(&connection.receiver_state, frame_type, len)
                }
            }

            Ok(())
        }

        fn process_accept<'a>(&'a mut self, session: W::Session, sender: &'a mut W) -> bool {
            if self.connections.len() < F {
                let receiver_state = Arc::new(Mutex::new(SharedReceiverState {
                    waker: None,
                    data: ReceiverData::None,
                }));

                let state = ConnectionState {
                    session,
                    receiver_state: receiver_state.clone(),
                };

                self.connections
                    .push(state)
                    .unwrap_or_else(|_| unreachable!());

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
            state: &Mutex<C::RawMutex, SharedReceiverState>,
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

        fn process_receive_close(state: &Mutex<C::RawMutex, SharedReceiverState>) {
            let mut shared = state.lock();

            shared.data = ReceiverData::Closed;

            if let Some(waker) = mem::replace(&mut shared.waker, None) {
                waker.wake();
            }
        }
    }

    impl<const N: usize, const F: usize, C, W> Drop for Processor<N, F, C, W>
    where
        C: RawCondvar + Send + Sync,
        C::RawMutex: Send + Sync,
        W: SenderFactory + SessionProvider,
        W::Sender: Send,
    {
        fn drop(&mut self) {
            self.process_accept_close();
        }
    }

    #[cfg(all(feature = "nightly", feature = "experimental"))]
    mod async_traits_impl {
        use core::future::Future;

        extern crate alloc;
        use alloc::borrow::ToOwned;
        use alloc::vec::Vec;

        use log::info;

        use crate::executor::asynch::Unblocker;
        use crate::mutex::RawCondvar;
        use crate::ws::{callback_server::*, *};

        use super::{AsyncAcceptor, AsyncConnection, AsyncReceiverFuture};

        impl<U, C, S> ErrorType for AsyncConnection<U, C, S>
        where
            C: RawCondvar,
            S: ErrorType,
        {
            type Error = S::Error;
        }

        impl<U, C, S> asynch::Sender for AsyncConnection<U, C, S>
        where
            U: Unblocker,
            C: RawCondvar,
            S: Sender + SessionProvider + Send + Clone + 'static,
            S::Error: Send + Sync + 'static,
        {
            type SendFuture<'a>
            = U::UnblockFuture<Result<(), S::Error>> where Self: 'a;

            fn send(&mut self, frame_type: FrameType, frame_data: &[u8]) -> Self::SendFuture<'_> {
                info!(
                    "Sending data (frame_type={:?}, frame_len={}) to WS connection {:?}",
                    frame_type,
                    frame_data.len(),
                    self.sender.session()
                );

                let mut sender = self.sender.clone();
                let frame_data: Vec<u8> = frame_data.to_owned();

                self.unblocker
                    .unblock(move || sender.send(frame_type, &frame_data))
            }
        }

        impl<C, S> asynch::Sender for AsyncConnection<(), C, S>
        where
            C: RawCondvar,
            S: Sender + SessionProvider + Send + Clone + 'static,
        {
            type SendFuture<'a>
            = impl Future<Output = Result<(), Self::Error>> where Self: 'a;

            fn send<'a>(
                &'a mut self,
                frame_type: FrameType,
                frame_data: &'a [u8],
            ) -> Self::SendFuture<'a> {
                async move { AsyncConnection::send(self, frame_type, frame_data).await }
            }
        }

        impl<U, C, S> asynch::Receiver for AsyncConnection<U, C, S>
        where
            U: Send,
            C: RawCondvar + Send + Sync,
            C::RawMutex: Send + Sync,
            S: ErrorType + Send,
        {
            type ReceiveFuture<'a>
            = AsyncReceiverFuture<'a, U, C, S> where Self: 'a;

            fn recv<'a>(&'a mut self, frame_data_buf: &'a mut [u8]) -> Self::ReceiveFuture<'a> {
                AsyncReceiverFuture {
                    receiver: self,
                    frame_data_buf,
                }
            }
        }

        impl<U, C, S> ErrorType for AsyncAcceptor<U, C, S>
        where
            C: RawCondvar + Send + Sync,
            C::RawMutex: Send + Sync,
            S: Send + ErrorType,
        {
            type Error = <S as ErrorType>::Error;
        }

        impl<U, C, S> asynch::server::Acceptor for AsyncAcceptor<U, C, S>
        where
            U: Unblocker + Clone + Send,
            C: RawCondvar + Send + Sync,
            C::RawMutex: Send + Sync,
            C::RawMutex: Send + Sync,
            S: Sender + SessionProvider + Send + Clone + 'static,
            S::Error: Send + Sync + 'static,
        {
            type Connection = AsyncConnection<U, C, S>;

            type AcceptFuture<'a>
            = &'a Self where Self: 'a;

            fn accept(&self) -> Self::AcceptFuture<'_> {
                self
            }
        }

        impl<C, S> asynch::server::Acceptor for AsyncAcceptor<(), C, S>
        where
            C: RawCondvar + Send + Sync,
            C::RawMutex: Send + Sync,
            C::RawMutex: Send + Sync,
            S: Sender + SessionProvider + Send + Clone + 'static,
        {
            type Connection = AsyncConnection<(), C, S>;

            type AcceptFuture<'a>
            = &'a Self where Self: 'a;

            fn accept(&self) -> Self::AcceptFuture<'_> {
                self
            }
        }
    }
}
