#[cfg(all(feature = "nightly", feature = "experimental"))]
pub mod asynch {
    use core::future::Future;

    pub trait Sender {
        type Data: Send;

        type SendFuture<'a>: Future + Send
        where
            Self: 'a;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_>;
    }

    impl<S> Sender for &mut S
    where
        S: Sender,
    {
        type Data = S::Data;

        type SendFuture<'a>
        = S::SendFuture<'a> where Self: 'a;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
            (*self).send(value)
        }
    }

    pub trait Receiver {
        type Data: Send;

        type RecvFuture<'a>: Future<Output = Self::Data> + Send
        where
            Self: 'a;

        fn recv(&mut self) -> Self::RecvFuture<'_>;
    }

    impl<R> Receiver for &mut R
    where
        R: Receiver,
    {
        type Data = R::Data;

        type RecvFuture<'a>
        = R::RecvFuture<'a> where Self: 'a;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            (*self).recv()
        }
    }

    pub mod adapt {
        use core::future::{pending, ready, Future, Pending, Ready};
        use core::marker::PhantomData;

        use futures::future::{select, Either};
        use futures::pin_mut;

        use crate::channel::asynch::{Receiver, Sender};

        pub fn adapt<C, T, F>(channel: C, adapter: F) -> AdapterChannel<C, F, T> {
            AdapterChannel::new(channel, adapter)
        }

        pub fn dummy<T: Send>() -> DummyChannel<T> {
            DummyChannel::new()
        }

        pub fn merge<A, B>(first: A, second: B) -> MergedChannel<A, B> {
            MergedChannel::new(first, second)
        }

        pub struct AdapterChannel<C, F, T> {
            inner: C,
            adapter: F,
            _input: PhantomData<fn() -> T>,
        }

        impl<C, F, T> AdapterChannel<C, F, T> {
            pub fn new(inner: C, adapter: F) -> Self {
                Self {
                    inner,
                    adapter,
                    _input: PhantomData,
                }
            }
        }

        impl<C, F, T> Sender for AdapterChannel<C, F, T>
        where
            C: Sender + Send + 'static,
            F: Fn(T) -> Option<C::Data> + Send + Sync,
            T: Send,
        {
            type Data = T;

            type SendFuture<'a>
            = impl Future<Output = ()> + Send where Self: 'a;

            fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
                let inner = &mut self.inner;
                let adapter = &self.adapter;

                send(inner, value, adapter)
            }
        }

        impl<C, F, T> Receiver for AdapterChannel<C, F, T>
        where
            C: Receiver + Send + 'static,
            F: Fn(C::Data) -> Option<T> + Send + Sync,
            T: Send,
        {
            type Data = T;

            type RecvFuture<'a>
            = impl Future<Output = Self::Data> + Send where Self: 'a;

            fn recv(&mut self) -> Self::RecvFuture<'_> {
                let inner = &mut self.inner;
                let adapter = &self.adapter;

                recv(inner, adapter)
            }
        }

        pub struct MergedChannel<A, B> {
            first: A,
            second: B,
        }

        impl<A, B> MergedChannel<A, B> {
            pub fn new(first: A, second: B) -> Self {
                Self { first, second }
            }

            pub fn and<T>(self, third: T) -> MergedChannel<Self, T> {
                MergedChannel::new(self, third)
            }
        }

        impl<A, B> Sender for MergedChannel<A, B>
        where
            A: Sender + Send + 'static,
            A::Data: Send + Sync + Clone,
            B: Sender<Data = A::Data> + Send + 'static,
        {
            type Data = A::Data;

            type SendFuture<'a>
            = impl Future<Output = ()> + Send where Self: 'a;

            fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
                async move { send_both(&mut self.first, &mut self.second, value).await }
            }
        }

        impl<A, B> Receiver for MergedChannel<A, B>
        where
            A: Receiver + Send + 'static,
            B: Receiver<Data = A::Data> + Send + 'static,
        {
            type Data = A::Data;

            type RecvFuture<'a>
            = impl Future<Output = Self::Data> + Send where Self: 'a;

            fn recv(&mut self) -> Self::RecvFuture<'_> {
                async move { recv_both(&mut self.first, &mut self.second).await }
            }
        }

        pub struct DummyChannel<T>(PhantomData<fn() -> T>);

        impl<T> DummyChannel<T> {
            pub fn new() -> Self {
                Self(PhantomData)
            }
        }

        impl<T> Default for DummyChannel<T> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<T> Sender for DummyChannel<T>
        where
            T: Send,
        {
            type Data = T;

            type SendFuture<'a>
            = Ready<()> where Self: 'a;

            fn send(&mut self, _value: Self::Data) -> Self::SendFuture<'_> {
                ready(())
            }
        }

        impl<T> Receiver for DummyChannel<T>
        where
            T: Send,
        {
            type Data = T;

            type RecvFuture<'a>
            = Pending<Self::Data> where Self: 'a;

            fn recv(&mut self) -> Self::RecvFuture<'_> {
                pending()
            }
        }

        pub async fn send<S, P>(sender: &mut S, value: P, adapter: &impl Fn(P) -> Option<S::Data>)
        where
            S: Sender,
        {
            if let Some(value) = adapter(value) {
                sender.send(value).await;
            }
        }

        pub async fn recv<R, P>(receiver: &mut R, adapter: &impl Fn(R::Data) -> Option<P>) -> P
        where
            R: Receiver,
        {
            loop {
                if let Some(value) = adapter(receiver.recv().await) {
                    return value;
                }
            }
        }

        pub async fn send_both<S1, S2>(sender1: &mut S1, sender2: &mut S2, value: S1::Data)
        where
            S1: Sender,
            S1::Data: Send + Clone,
            S2: Sender<Data = S1::Data>,
        {
            sender1.send(value.clone()).await;
            sender2.send(value).await;
        }

        pub async fn recv_both<R1, R2>(receiver1: &mut R1, receiver2: &mut R2) -> R1::Data
        where
            R1: Receiver,
            R2: Receiver<Data = R1::Data>,
        {
            let f1 = receiver1.recv();
            let f2 = receiver2.recv();

            pin_mut!(f1, f2);

            match select(f1, f2).await {
                Either::Left((r1, _)) => r1,
                Either::Right((r2, _)) => r2,
            }
        }
    }
}
