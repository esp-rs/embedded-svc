pub mod adapt {
    use core::future::Future;
    use core::marker::PhantomData;

    use crate::errors::Errors;

    use crate::channel::asyncs::{Receiver, Sender};

    pub fn sender<S, P, F>(sender: S, adapter: F) -> impl Sender<Data = P>
    where
        S: Sender,
        F: Fn(P) -> Option<S::Data>,
    {
        SenderAdapter::new(sender, adapter)
    }

    pub fn receiver<R, P, F>(receiver: R, adapter: F) -> impl Receiver<Data = P>
    where
        R: Receiver,
        F: Fn(R::Data) -> Option<P>,
    {
        ReceiverAdapter::new(receiver, adapter)
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
        S: Sender,
        F: Fn(P) -> Option<S::Data>,
    {
        type Data = P;

        type SendFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>>;

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
        R: Receiver,
        F: Fn(R::Data) -> Option<P>,
    {
        type Data = P;

        type RecvFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<Self::Data, Self::Error>>;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            let inner_receiver = &mut self.inner_receiver;
            let adapter = &self.adapter;

            recv(inner_receiver, adapter)
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
}
