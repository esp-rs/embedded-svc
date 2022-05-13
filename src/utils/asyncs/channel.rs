pub mod adapt {
    use core::future::Future;
    use core::marker::PhantomData;

    use crate::errors::Errors;

    use crate::channel::asyncs::{Receiver, Sender};

    pub fn sender<S, P, F>(sender: S, adapter: F) -> impl Sender<Data = P>
    where
        S: Sender + Send + 'static,
        P: Send,
        F: Fn(P) -> Option<S::Data> + Send + Sync,
    {
        SenderAdapter::new(sender, adapter)
    }

    pub fn receiver<R, P, F>(receiver: R, adapter: F) -> impl Receiver<Data = P>
    where
        R: Receiver + Send + 'static,
        P: Send,
        F: Fn(R::Data) -> Option<P> + Send + Sync,
    {
        ReceiverAdapter::new(receiver, adapter)
    }

    pub fn all_senders<S, const N: usize>(senders: [S; N]) -> impl Sender<Data = S::Data>
    where
        S: Sender + Send + 'static,
        S::Data: Send + Clone,
    {
        senders
    }

    #[cfg(feature = "heapless")]
    pub fn all_receivers<R, const N: usize>(receivers: [R; N]) -> impl Receiver<Data = R::Data>
    where
        R: Receiver + Send + 'static,
    {
        receivers
    }

    struct SenderAdapter<S, F, P> {
        inner_sender: S,
        adapter: F,
        _input: PhantomData<fn() -> P>,
    }

    impl<S, F, P> SenderAdapter<S, F, P> {
        pub fn new(inner_sender: S, adapter: F) -> Self {
            Self {
                inner_sender,
                adapter,
                _input: PhantomData,
            }
        }
    }

    impl<S, F, P> Errors for SenderAdapter<S, F, P>
    where
        S: Errors,
    {
        type Error = S::Error;
    }

    impl<S, F, P> Sender for SenderAdapter<S, F, P>
    where
        S: Sender + Send + 'static,
        F: Fn(P) -> Option<S::Data> + Send + Sync,
        P: Send,
    {
        type Data = P;

        type SendFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>> + Send;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
            let inner_sender = &mut self.inner_sender;
            let adapter = &self.adapter;

            send(inner_sender, value, adapter)
        }
    }

    struct ReceiverAdapter<R, F, P> {
        inner_receiver: R,
        adapter: F,
        _output: PhantomData<fn() -> P>,
    }

    impl<R, F, P> ReceiverAdapter<R, F, P> {
        pub fn new(inner_receiver: R, adapter: F) -> Self {
            Self {
                inner_receiver,
                adapter,
                _output: PhantomData,
            }
        }
    }

    impl<R, F, P> Errors for ReceiverAdapter<R, F, P>
    where
        R: Errors,
    {
        type Error = R::Error;
    }

    impl<R, F, P> Receiver for ReceiverAdapter<R, F, P>
    where
        R: Receiver + Send + 'static,
        F: Fn(R::Data) -> Option<P> + Send + Sync,
        P: Send,
    {
        type Data = P;

        type RecvFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<Self::Data, Self::Error>> + Send;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            let inner_receiver = &mut self.inner_receiver;
            let adapter = &self.adapter;

            recv(inner_receiver, adapter)
        }
    }

    impl<R, const N: usize> Errors for [R; N]
    where
        R: Errors,
    {
        type Error = R::Error;
    }

    impl<S, const N: usize> Sender for [S; N]
    where
        S: Sender + Send + 'static,
        S::Data: Send + Clone,
    {
        type Data = S::Data;

        type SendFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>> + Send;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
            send_all(self, value)
        }
    }

    #[cfg(feature = "heapless")]
    impl<R, const N: usize> Receiver for [R; N]
    where
        R: Receiver + Send + 'static,
    {
        type Data = R::Data;

        type RecvFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<Self::Data, Self::Error>> + Send;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            recv_all(self)
        }
    }

    pub async fn send<S, P>(
        sender: &mut S,
        value: P,
        adapter: &impl Fn(P) -> Option<S::Data>,
    ) -> Result<(), S::Error>
    where
        S: Sender + Errors,
    {
        if let Some(value) = adapter(value) {
            sender.send(value).await
        } else {
            Ok(())
        }
    }

    pub async fn recv<R, P>(
        receiver: &mut R,
        adapter: &impl Fn(R::Data) -> Option<P>,
    ) -> Result<P, R::Error>
    where
        R: Receiver + Errors,
    {
        loop {
            if let Some(value) = adapter(receiver.recv().await?) {
                return Ok(value);
            }
        }
    }

    pub async fn send_all<S, const N: usize>(senders: &mut [S; N], value: S::Data) -> Result<(), S::Error>
    where
        S: Sender + Errors,
        S::Data: Send + Clone,
    {
        for sender in senders {
            let value = value.clone();
            sender.send(value).await?;
        }

        Ok(())
    }

    #[cfg(feature = "heapless")]
    pub async fn recv_all<R, const N: usize>(receivers: &mut [R; N]) -> Result<R::Data, R::Error>
    where
        R: Receiver + Errors,
    {
        let (data, _) = crate::utils::asyncs::select::select_all_hvec(
            receivers
                .iter_mut()
                .map(|r| r.recv())
                .collect::<heapless::Vec<_, N>>())
            .await;

        data
    }
}
