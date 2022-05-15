pub mod adapt {
    use core::convert::Infallible;
    use core::future::{pending, ready, Future, Pending, Ready};
    use core::marker::PhantomData;

    use crate::errors::{EitherError, Errors};

    use crate::channel::asyncs::{Receiver, Sender};
    use crate::utils::asyncs::select::{select, Either};

    pub fn sender<S, P, F>(sender: S, adapter: F) -> impl Sender<Data = P>
    where
        S: Sender + Send + 'static,
        P: Send,
        F: Fn(P) -> Option<S::Data> + Send + Sync,
    {
        Adapter::new(sender, adapter)
    }

    pub fn receiver<R, P, F>(receiver: R, adapter: F) -> impl Receiver<Data = P>
    where
        R: Receiver + Send + 'static,
        P: Send,
        F: Fn(R::Data) -> Option<P> + Send + Sync,
    {
        Adapter::new(receiver, adapter)
    }

    pub fn dummy<T: Send>() -> impl Sender<Data = T> + Receiver<Data = T> {
        Dummy(PhantomData)
    }

    pub fn both<A, B>(first: A, second: B) -> Both<A, B> {
        Both::new(first, second)
    }

    struct Adapter<I, F, T> {
        inner: I,
        adapter: F,
        _input: PhantomData<fn() -> T>,
    }

    impl<I, F, T> Adapter<I, F, T> {
        pub fn new(inner: I, adapter: F) -> Self {
            Self {
                inner,
                adapter,
                _input: PhantomData,
            }
        }
    }

    impl<I, F, T> Errors for Adapter<I, F, T>
    where
        I: Errors,
    {
        type Error = I::Error;
    }

    impl<I, F, T> Sender for Adapter<I, F, T>
    where
        I: Sender + Send + 'static,
        F: Fn(T) -> Option<I::Data> + Send + Sync,
        T: Send,
    {
        type Data = T;

        type SendFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>> + Send;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
            let inner = &mut self.inner;
            let adapter = &self.adapter;

            send(inner, value, adapter)
        }
    }

    impl<I, F, T> Receiver for Adapter<I, F, T>
    where
        I: Receiver + Send + 'static,
        F: Fn(I::Data) -> Option<T> + Send + Sync,
        T: Send,
    {
        type Data = T;

        type RecvFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<Self::Data, Self::Error>> + Send;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            let inner = &mut self.inner;
            let adapter = &self.adapter;

            recv(inner, adapter)
        }
    }

    pub struct Both<A, B> {
        first: A,
        second: B,
    }

    impl<A, B> Both<A, B> {
        pub fn new(first: A, second: B) -> Self {
            Self { first, second }
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
            async move { send_both(&mut self.first, &mut self.second, value).await }
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
            async move { recv_both(&mut self.first, &mut self.second).await }
        }
    }

    struct Dummy<T>(PhantomData<fn() -> T>);

    impl<T> Errors for Dummy<T> {
        type Error = Infallible;
    }

    impl<T> Sender for Dummy<T>
    where
        T: Send,
    {
        type Data = T;

        type SendFuture<'a>
        where
            Self: 'a,
        = Ready<Result<(), Self::Error>>;

        fn send(&mut self, _value: Self::Data) -> Self::SendFuture<'_> {
            ready(Ok(()))
        }
    }

    impl<T> Receiver for Dummy<T>
    where
        T: Send,
    {
        type Data = T;

        type RecvFuture<'a>
        where
            Self: 'a,
        = Pending<Result<Self::Data, Self::Error>>;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            pending()
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

    pub async fn send_both<S1, S2>(
        sender1: &mut S1,
        sender2: &mut S2,
        value: S1::Data,
    ) -> Result<(), EitherError<S1::Error, S2::Error>>
    where
        S1: Sender + Errors,
        S1::Data: Send + Clone,
        S2: Sender<Data = S1::Data> + Errors,
    {
        sender1
            .send(value.clone())
            .await
            .map_err(EitherError::First)?;
        sender2.send(value).await.map_err(EitherError::Second)?;

        Ok(())
    }

    pub async fn recv_both<R1, R2>(
        receiver1: &mut R1,
        receiver2: &mut R2,
    ) -> Result<R1::Data, EitherError<R1::Error, R2::Error>>
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
