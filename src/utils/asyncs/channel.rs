pub mod adapt {
    use core::future::Future;
    use core::marker::PhantomData;

    use crate::errors::{Errors, EitherError};

    use crate::channel::asyncs::{Receiver, Sender};
    use crate::utils::asyncs::select::{select, Either};

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

    pub fn both<A, B>(first: A, second: B) -> Both<A, B> {
        Both::new(first, second)
    }

    pub struct Both<A, B> {
        first: A,
        second: B,
    }

    impl<A, B> Both<A, B> {
        pub fn new(first: A, second: B) -> Self {
            Self {
                first,
                second,
            }
        }

        pub fn and<T>(self, third: T) -> Both<Self, T> {
            Both::new(self, third)
        }
    }

    impl<A, B> Errors for Both<A, B>
    where
        A: Errors,
        B: Errors,
    {
        type Error = EitherError<A::Error, B::Error>;
    }

    impl<A, B> Sender for Both<A, B>
    where
        A: Sender + Send + 'static,
        A::Data: Send + Sync + Clone,
        B: Sender<Data = A::Data> + Send + 'static,
    {
        type Data = A::Data;

        type SendFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>> + Send;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
            async move {
                send_both(&mut self.first, &mut self.second, value).await
            }
        }
    }

    impl<A, B> Receiver for Both<A, B>
    where
        A: Receiver + Send + 'static,
        A: Errors,
        B: Receiver<Data = A::Data> + Send + 'static,
        B: Errors<Error = A::Error>,
    {
        type Data = A::Data;

        type RecvFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<Self::Data, Self::Error>> + Send;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            async move {
                recv_both(&mut self.first, &mut self.second).await
            }
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

    pub async fn send_both<S1, S2>(sender1: &mut S1, sender2: &mut S2, value: S1::Data) -> Result<(), EitherError<S1::Error, S2::Error>>
    where
        S1: Sender + Errors,
        S1::Data: Send + Clone,
        S2: Sender<Data = S1::Data> + Errors,
    {
        sender1.send(value.clone()).await.map_err(EitherError::First)?;
        sender2.send(value).await.map_err(EitherError::Second)?;

        Ok(())
    }

    pub async fn recv_both<R1, R2>(receiver1: &mut R1, receiver2: &mut R2) -> Result<R1::Data, EitherError<R1::Error, R2::Error>>
    where
        R1: Receiver + Errors,
        R2: Receiver<Data = R1::Data> + Errors,
    {
        let receiver1 = receiver1.recv();
        let receiver2 = receiver2.recv();

        //pin_mut!(receiver1, receiver2);

        match select(receiver1, receiver2).await {
            Either::First(r) => r.map_err(EitherError::First),
            Either::Second(r) => r.map_err(EitherError::Second),
        }
    }
}
