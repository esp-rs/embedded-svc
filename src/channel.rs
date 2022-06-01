#[cfg(feature = "experimental")]
pub mod asyncs {
    use core::future::Future;

    pub trait Sender {
        type Data: Send;

        type SendFuture<'a>: Future + Send
        where
            Self: 'a;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_>;
    }

    pub trait Receiver {
        type Data: Send;

        type RecvFuture<'a>: Future<Output = Self::Data> + Send
        where
            Self: 'a;

        fn recv(&mut self) -> Self::RecvFuture<'_>;
    }
}
