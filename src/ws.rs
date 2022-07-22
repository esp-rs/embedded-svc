use core::fmt::Debug;

pub trait ErrorType {
    type Error: Debug;
}

impl<E> ErrorType for &E
where
    E: ErrorType,
{
    type Error = E::Error;
}

impl<E> ErrorType for &mut E
where
    E: ErrorType,
{
    type Error = E::Error;
}

pub type Fragmented = bool;
pub type Final = bool;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FrameType {
    Text(Fragmented),
    Binary(Fragmented),
    Ping,
    Pong,
    Close,
    SocketClose,
    Continue(Final),
}

impl FrameType {
    pub fn is_fragmented(&self) -> bool {
        match self {
            Self::Text(fragmented) | Self::Binary(fragmented) => *fragmented,
            Self::Continue(_) => true,
            _ => false,
        }
    }

    pub fn is_final(&self) -> bool {
        match self {
            Self::Text(fragmented) | Self::Binary(fragmented) => !*fragmented,
            Self::Continue(final_) => *final_,
            _ => true,
        }
    }
}

pub trait Receiver: ErrorType {
    fn recv(&mut self, frame_data_buf: &mut [u8]) -> Result<(FrameType, usize), Self::Error>;
}

impl<R> Receiver for &mut R
where
    R: Receiver,
{
    fn recv(&mut self, frame_data_buf: &mut [u8]) -> Result<(FrameType, usize), Self::Error> {
        (*self).recv(frame_data_buf)
    }
}

pub trait Sender: ErrorType {
    fn send(&mut self, frame_type: FrameType, frame_data: Option<&[u8]>)
        -> Result<(), Self::Error>;
}

impl<S> Sender for &mut S
where
    S: Sender,
{
    fn send(
        &mut self,
        frame_type: FrameType,
        frame_data: Option<&[u8]>,
    ) -> Result<(), Self::Error> {
        (*self).send(frame_type, frame_data)
    }
}

pub mod client {
    use crate::io::Io;

    pub use super::*;

    pub trait WsClient: Io {
        type Connection<'a>: Sender<Error = Self::Error> + Receiver<Error = Self::Error>
        where
            Self: 'a;

        fn request_ws<'a>(
            &'a mut self,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<Self::Connection<'a>, Self::Error>;
    }
}

pub mod server {
    pub use super::*;

    pub trait Acceptor: ErrorType {
        type Connection<'m>: Sender<Error = Self::Error> + Receiver<Error = Self::Error> + Send
        where
            Self: 'm;

        fn accept(&self) -> Result<Option<Self::Connection<'_>>, Self::Error>;
    }

    impl<A> Acceptor for &A
    where
        A: Acceptor,
    {
        type Connection<'m>
        where
            Self: 'm,
        = A::Connection<'m>;

        fn accept(&self) -> Result<Option<Self::Connection<'_>>, Self::Error> {
            (**self).accept()
        }
    }

    impl<A> Acceptor for &mut A
    where
        A: Acceptor,
    {
        type Connection<'m>
        where
            Self: 'm,
        = A::Connection<'m>;

        fn accept(&self) -> Result<Option<Self::Connection<'_>>, Self::Error> {
            (**self).accept()
        }
    }

    pub trait SessionProvider {
        type Session: Clone + Send + PartialEq + Debug;

        fn session(&self) -> Self::Session;

        fn is_new(&self) -> bool;
        fn is_closed(&self) -> bool;
    }

    pub trait SenderFactory: ErrorType {
        type Sender: Sender<Error = Self::Error>;

        fn create(&self) -> Result<Self::Sender, Self::Error>;
    }

    // pub trait Registry {
    //     type Error: Debug;

    //     type SendReceiveError: Debug;

    //     type Connection: Receiver<Error = Self::SendReceiveError>
    //         + Sender<Error = Self::SendReceiveError>
    //         + SessionProvider
    //         + SenderFactory<Error = Self::SendReceiveError>;

    //     fn handle_ws<H, E>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
    //     where
    //         H: for<'a> Fn(&'a mut Self::Connection) -> Result<(), E> + Send + 'static,
    //         E: Debug;
    // }
}

pub mod asynch {
    use core::future::Future;

    use crate::executor::asynch::{Blocker, Blocking, TrivialAsync};

    pub use super::{ErrorType, Fragmented, FrameType};

    pub trait Receiver: ErrorType {
        type ReceiveFuture<'a>: Future<Output = Result<(FrameType, usize), Self::Error>> + Send
        where
            Self: 'a;

        fn recv<'a>(&'a mut self, frame_data_buf: &'a mut [u8]) -> Self::ReceiveFuture<'a>;
    }

    impl<R> Receiver for &mut R
    where
        R: Receiver,
    {
        type ReceiveFuture<'a>
        where
            Self: 'a,
        = R::ReceiveFuture<'a>;

        fn recv<'a>(&'a mut self, frame_data_buf: &'a mut [u8]) -> Self::ReceiveFuture<'a> {
            (*self).recv(frame_data_buf)
        }
    }

    pub trait Sender: ErrorType {
        type SendFuture<'a>: Future<Output = Result<(), Self::Error>> + Send
        where
            Self: 'a;

        fn send<'a>(
            &'a mut self,
            frame_type: FrameType,
            frame_data: Option<&'a [u8]>,
        ) -> Self::SendFuture<'a>;
    }

    impl<S> Sender for &mut S
    where
        S: Sender,
    {
        type SendFuture<'a>
        where
            Self: 'a,
        = S::SendFuture<'a>;

        fn send<'a>(
            &'a mut self,
            frame_type: FrameType,
            frame_data: Option<&'a [u8]>,
        ) -> Self::SendFuture<'a> {
            (*self).send(frame_type, frame_data)
        }
    }

    impl<B, E> ErrorType for Blocking<B, E>
    where
        E: ErrorType,
    {
        type Error = E::Error;
    }

    impl<B, S> super::Sender for Blocking<B, S>
    where
        B: Blocker,
        S: Sender,
    {
        fn send(
            &mut self,
            frame_type: FrameType,
            frame_data: Option<&[u8]>,
        ) -> Result<(), Self::Error> {
            self.0.block_on(self.1.send(frame_type, frame_data))
        }
    }

    impl<B, R> super::Receiver for Blocking<B, R>
    where
        B: Blocker,
        R: Receiver,
    {
        fn recv(&mut self, frame_data_buf: &mut [u8]) -> Result<(FrameType, usize), Self::Error> {
            self.0.block_on(self.1.recv(frame_data_buf))
        }
    }

    impl<S> Sender for TrivialAsync<S>
    where
        S: super::Sender + Send,
    {
        type SendFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>>;

        fn send<'a>(
            &'a mut self,
            frame_type: FrameType,
            frame_data: Option<&'a [u8]>,
        ) -> Self::SendFuture<'a> {
            async move { self.1.send(frame_type, frame_data) }
        }
    }

    impl<R> Receiver for TrivialAsync<R>
    where
        R: super::Receiver + Send,
    {
        type ReceiveFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(FrameType, usize), Self::Error>>;

        fn recv<'a>(&'a mut self, frame_data_buf: &'a mut [u8]) -> Self::ReceiveFuture<'a> {
            async move { self.1.recv(frame_data_buf) }
        }
    }

    pub mod client {
        use crate::io::Io;

        pub use super::*;

        pub trait WsClient: Io {
            type Connection<'a>: Sender<Error = Self::Error> + Receiver<Error = Self::Error>
            where
                Self: 'a;

            type RequestWsFuture<'a>: Future<Output = Result<Self::Connection<'a>, Self::Error>>
            where
                Self: 'a;

            fn request_ws<'a>(
                &'a mut self,
                uri: &'a str,
                headers: &'a [(&'a str, &'a str)],
            ) -> Self::RequestWsFuture<'a>;
        }

        impl<C> WsClient for &mut C
        where
            C: WsClient,
        {
            type Connection<'a>
            where
                Self: 'a,
            = C::Connection<'a>;

            type RequestWsFuture<'a>
            where
                Self: 'a,
            = C::RequestWsFuture<'a>;

            fn request_ws<'a>(
                &'a mut self,
                uri: &'a str,
                headers: &'a [(&'a str, &'a str)],
            ) -> Self::RequestWsFuture<'a> {
                (*self).request_ws(uri, headers)
            }
        }

        impl<B, C> crate::ws::client::WsClient for Blocking<B, C>
        where
            B: Blocker + Clone,
            C: WsClient,
        {
            type Connection<'a>
            where
                Self: 'a,
            = Blocking<B, C::Connection<'a>>;

            fn request_ws<'a>(
                &'a mut self,
                uri: &'a str,
                headers: &'a [(&'a str, &'a str)],
            ) -> Result<Self::Connection<'a>, Self::Error> {
                let connection = self.0.block_on(self.1.request_ws(uri, headers))?;

                Ok(Blocking::new(self.0.clone(), connection))
            }
        }

        impl<C> WsClient for TrivialAsync<C>
        where
            C: crate::ws::client::WsClient,
            for<'m> C::Connection<'m>: Send,
        {
            type Connection<'a>
            where
                Self: 'a,
            = TrivialAsync<C::Connection<'a>>;

            type RequestWsFuture<'a>
            where
                Self: 'a,
            = impl Future<Output = Result<Self::Connection<'a>, Self::Error>>;

            fn request_ws<'a>(
                &'a mut self,
                uri: &'a str,
                headers: &'a [(&'a str, &'a str)],
            ) -> Self::RequestWsFuture<'a> {
                async move { Ok(TrivialAsync::new_async(self.1.request_ws(uri, headers)?)) }
            }
        }
    }

    pub mod server {
        use core::future::Future;

        use crate::executor::asynch::{Blocker, Blocking, TrivialAsync};

        pub use super::{ErrorType, Receiver, Sender};

        pub trait Acceptor: ErrorType {
            type Connection<'m>: Sender<Error = Self::Error> + Receiver<Error = Self::Error> + Send
            where
                Self: 'm;

            type AcceptFuture<'a>: Future<Output = Result<Option<Self::Connection<'a>>, Self::Error>>
                + Send
            where
                Self: 'a;

            fn accept(&self) -> Self::AcceptFuture<'_>;
        }

        impl<A> Acceptor for &A
        where
            A: Acceptor,
        {
            type Connection<'m>
            where
                Self: 'm,
            = A::Connection<'m>;

            type AcceptFuture<'a>
            where
                Self: 'a,
            = A::AcceptFuture<'a>;

            fn accept(&self) -> Self::AcceptFuture<'_> {
                (*self).accept()
            }
        }

        impl<B, A> crate::ws::server::Acceptor for Blocking<B, A>
        where
            B: Blocker + Clone + Send,
            A: Acceptor,
        {
            type Connection<'m>
            where
                Self: 'm,
            = Blocking<B, A::Connection<'m>>;

            fn accept(&self) -> Result<Option<Self::Connection<'_>>, Self::Error> {
                let r = self.0.block_on(self.1.accept())?;

                Ok(r.map(|connection| Blocking::new(self.0.clone(), connection)))
            }
        }

        impl<A> Acceptor for TrivialAsync<A>
        where
            A: crate::ws::server::Acceptor + Send + Sync,
        {
            type Connection<'m>
            where
                Self: 'm,
            = TrivialAsync<A::Connection<'m>>;

            type AcceptFuture<'a>
            where
                Self: 'a,
            = impl Future<Output = Result<Option<Self::Connection<'a>>, Self::Error>>;

            fn accept(&self) -> Self::AcceptFuture<'_> {
                async move { Ok(self.1.accept()?.map(TrivialAsync::new_async)) }
            }
        }
    }
}
