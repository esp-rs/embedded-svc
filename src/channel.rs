pub mod nonblocking {
    use core::future::Future;

    use crate::service::Service;

    pub trait Sender: Service {
        type Data;

        type SendFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_>;
    }

    pub trait Receiver: Service {
        type Data;

        type RecvFuture<'a>: Future<Output = Result<Self::Data, Self::Error>>
        where
            Self: 'a;

        fn recv(&mut self) -> Self::RecvFuture<'_>;
    }

    pub trait Channel: Service {
        type Data;

        type Sender: Sender<Data = Self::Data, Error = Self::Error>;
        type Receiver: Receiver<Data = Self::Data, Error = Self::Error>;

        fn split(self) -> (Self::Sender, Self::Receiver);
    }
}
