#[cfg(feature = "experimental")]
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
        where
            Self: 'a,
        = S::SendFuture<'a>;

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
        where
            Self: 'a,
        = R::RecvFuture<'a>;

        fn recv(&mut self) -> Self::RecvFuture<'_> {
            (*self).recv()
        }
    }
}
