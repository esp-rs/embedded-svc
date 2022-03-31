#[cfg(feature = "experimental")]
pub mod asyncs {
    use core::future::Future;

    use crate::errors::Errors;

    pub trait Sender: Errors {
        type Data;

        type SendFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_>;
    }

    pub trait Receiver: Errors {
        type Data;

        type RecvFuture<'a>: Future<Output = Result<Self::Data, Self::Error>>
        where
            Self: 'a;

        fn recv(&mut self) -> Self::RecvFuture<'_>;
    }
}
