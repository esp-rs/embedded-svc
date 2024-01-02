#[cfg(feature = "alloc")]
pub mod server {
    use core::future::Future;
    use core::marker::PhantomData;
    use core::pin::Pin;
    use core::slice;
    use core::task::{Context, Poll, Waker};

    extern crate alloc;
    use alloc::sync::Arc;

    use heapless;

    use crate::log::svc_log;
    use crate::utils::mutex::{Condvar, Mutex, RawCondvar};
    use crate::ws::{callback_server::*, *};

    #[allow(unused_imports)]
    pub use async_traits_impl::*;

    #[allow(dead_code)]
    pub struct AsyncSender<U, S> {
        unblocker: U,
        sender: S,
    }

    impl<S> AsyncSender<(), S>
    where
        S: Sender + SessionProvider,
    {
        pub fn send_blocking(
            &mut self,
            frame_type: FrameType,
            frame_data: &[u8],
        ) -> Result<(), S::Error> {
            svc_log!(
                debug,
                "Sending data (frame_type={:?}, frame_len={}) to WS connection {:?}",
                frame_type,
                frame_data.len(),
                self.sender.session()
            );

            self.sender.send(frame_type, frame_data)
        }
    }

    impl<U, S> AsyncSender<U, S>
    where
        U: crate::utils::asyncify::Unblocker,
        S: Sender + SessionProvider + Send + Clone,
        S::Error: Send + Sync,
    {
        pub async fn send(
            &mut self,
            frame_type: FrameType,
            frame_data: &[u8],
        ) -> Result<(), S::Error> {
            #[cfg(not(feature = "std"))]
            use alloc::borrow::ToOwned;

            svc_log!(
                debug,
                "Sending data (frame_type={:?}, frame_len={}) to WS connection {:?}",
                frame_type,
                frame_data.len(),
                self.sender.session()
            );

            let mut sender = self.sender.clone();
            let frame_data: alloc::vec::Vec<u8> = frame_data.to_owned();

            self.unblocker
                .unblock(move || sender.send(frame_type, &frame_data))
                .await
        }
    }

    #[allow(dead_code)]
    pub struct AsyncReceiver<C, E>
    where
        C: RawCondvar,
    {
        shared: Arc<Mutex<C::RawMutex, SharedReceiverState>>,
        condvar: Arc<Condvar<C>>,
        _ep: PhantomData<fn() -> E>,
    }

    impl<C, E> AsyncReceiver<C, E>
    where
        C: RawCondvar + Send + Sync,
        C::RawMutex: Send + Sync,
    {
        pub async fn recv(&mut self, frame_data_buf: &mut [u8]) -> Result<(FrameType, usize), E> {
            AsyncReceiverFuture {
                receiver: self,
                frame_data_buf,
                _ep: PhantomData,
            }
            .await
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

    struct AsyncReceiverFuture<'a, C, E>
    where
        C: RawCondvar,
    {
        receiver: &'a mut AsyncReceiver<C, E>,
        frame_data_buf: &'a mut [u8],
        _ep: PhantomData<fn() -> E>,
    }

    impl<'a, C, E> Future for AsyncReceiverFuture<'a, C, E>
    where
        C: RawCondvar,
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
        S: Sender + SessionProvider + Send,
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
        S: Sender + Send + Clone,
    {
        type Output = Result<
            (AsyncSender<U, S>, AsyncReceiver<C, <S as ErrorType>::Error>),
            <S as ErrorType>::Error,
        >;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut accept = self.accept.lock();

            match accept.data.take() {
                Some(Some((shared, sender))) => {
                    let sender = AsyncSender {
                        unblocker: self.unblocker.clone(),
                        sender,
                    };

                    let receiver = AsyncReceiver {
                        shared,
                        condvar: self.condvar.clone(),
                        _ep: PhantomData,
                    };

                    self.condvar.notify_all();

                    Poll::Ready(Ok((sender, receiver)))
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

                svc_log!(info, "New WS connection {:?}", session);

                if !self.process_accept(session, connection) {
                    return connection.send(FrameType::Close, &[]);
                }
            } else if connection.is_closed() {
                let session = connection.session();

                if let Some(index) = self
                    .connections
                    .iter()
                    .enumerate()
                    .find_map(|(index, conn)| (conn.session == session).then_some(index))
                {
                    let conn = self.connections.swap_remove(index);

                    Self::process_receive_close(&conn.receiver_state);
                    svc_log!(info, "Closed WS connection {:?}", session);
                }
            } else {
                let session = connection.session();
                let (frame_type, len) = connection.recv(&mut self.frame_data_buf)?;

                svc_log!(
                    debug,
                    "Incoming data (frame_type={:?}, frame_len={}) from WS connection {:?}",
                    frame_type,
                    len,
                    session
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

        fn process_accept<'a>(&'a mut self, session: W::Session, sender: &'a W) -> bool {
            if self.connections.len() < N {
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

                if let Some(waker) = accept.waker.take() {
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

            if let Some(waker) = shared.waker.take() {
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

            if let Some(waker) = accept.waker.take() {
                waker.wake();
            }
        }

        fn process_receive_close(state: &Mutex<C::RawMutex, SharedReceiverState>) {
            let mut shared = state.lock();

            shared.data = ReceiverData::Closed;

            if let Some(waker) = shared.waker.take() {
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

    mod async_traits_impl {
        use core::fmt::Debug;

        use crate::utils::asyncify::Unblocker;
        use crate::utils::mutex::RawCondvar;
        use crate::ws::{callback_server::*, *};

        use super::{AsyncAcceptor, AsyncReceiver, AsyncSender};

        impl<U, S> ErrorType for AsyncSender<U, S>
        where
            S: ErrorType,
        {
            type Error = S::Error;
        }

        impl<U, S> asynch::Sender for AsyncSender<U, S>
        where
            U: Unblocker,
            S: Sender + SessionProvider + Send + Clone,
            S::Error: Send + Sync + 'static,
        {
            async fn send(
                &mut self,
                frame_type: FrameType,
                frame_data: &[u8],
            ) -> Result<(), Self::Error> {
                AsyncSender::send(self, frame_type, frame_data).await
            }
        }

        impl<S> asynch::Sender for AsyncSender<(), S>
        where
            S: Sender + SessionProvider,
        {
            async fn send(
                &mut self,
                frame_type: FrameType,
                frame_data: &[u8],
            ) -> Result<(), Self::Error> {
                AsyncSender::send_blocking(self, frame_type, frame_data)
            }
        }

        impl<C, E> ErrorType for AsyncReceiver<C, E>
        where
            C: RawCondvar,
            E: Debug,
        {
            type Error = E;
        }

        impl<C, E> asynch::Receiver for AsyncReceiver<C, E>
        where
            C: RawCondvar + Send + Sync,
            C::RawMutex: Send + Sync,
            E: Debug,
        {
            async fn recv(
                &mut self,
                frame_data_buf: &mut [u8],
            ) -> Result<(FrameType, usize), Self::Error> {
                AsyncReceiver::recv(self, frame_data_buf).await
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
            S: Sender + SessionProvider + Send + Clone,
            S::Error: Send + Sync + 'static,
        {
            type Sender<'a> = AsyncSender<U, S> where U: 'a, C: 'a, S: 'a;
            type Receiver<'a> = AsyncReceiver<C, S::Error> where U: 'a, S: 'a, C: 'a;

            async fn accept(&self) -> Result<(Self::Sender<'_>, Self::Receiver<'_>), Self::Error> {
                self.await
            }
        }

        impl<C, S> asynch::server::Acceptor for AsyncAcceptor<(), C, S>
        where
            C: RawCondvar + Send + Sync,
            C::RawMutex: Send + Sync,
            C::RawMutex: Send + Sync,
            S: Sender + SessionProvider + Send + Clone,
        {
            type Sender<'a> = AsyncSender<(), S> where C: 'a, S: 'a;
            type Receiver<'a> = AsyncReceiver<C, S::Error> where C: 'a, S: 'a;

            async fn accept(&self) -> Result<(Self::Sender<'_>, Self::Receiver<'_>), Self::Error> {
                self.await
            }
        }
    }
}
