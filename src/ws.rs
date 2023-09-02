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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    fn send(&mut self, frame_type: FrameType, frame_data: &[u8]) -> Result<(), Self::Error>;
}

impl<S> Sender for &mut S
where
    S: Sender,
{
    fn send(&mut self, frame_type: FrameType, frame_data: &[u8]) -> Result<(), Self::Error> {
        (*self).send(frame_type, frame_data)
    }
}

pub mod server {
    pub use super::*;

    pub trait Acceptor: ErrorType {
        type Connection: Sender<Error = Self::Error> + Receiver<Error = Self::Error>;

        fn accept(&self) -> Result<Self::Connection, Self::Error>;
    }

    impl<A> Acceptor for &A
    where
        A: Acceptor,
    {
        type Connection = A::Connection;

        fn accept(&self) -> Result<Self::Connection, Self::Error> {
            (*self).accept()
        }
    }
}

pub mod callback_server {
    pub use super::*;

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
}

#[cfg(feature = "nightly")]
pub mod asynch {
    use crate::executor::asynch::{Blocker, Blocking, TrivialUnblocking};

    pub use super::{ErrorType, Fragmented, FrameType};

    pub trait Receiver: ErrorType {
        async fn recv(
            &mut self,
            frame_data_buf: &mut [u8],
        ) -> Result<(FrameType, usize), Self::Error>;
    }

    impl<R> Receiver for &mut R
    where
        R: Receiver,
    {
        async fn recv(
            &mut self,
            frame_data_buf: &mut [u8],
        ) -> Result<(FrameType, usize), Self::Error> {
            (*self).recv(frame_data_buf).await
        }
    }

    pub trait Sender: ErrorType {
        async fn send(
            &mut self,
            frame_type: FrameType,
            frame_data: &[u8],
        ) -> Result<(), Self::Error>;
    }

    impl<S> Sender for &mut S
    where
        S: Sender,
    {
        async fn send(
            &mut self,
            frame_type: FrameType,
            frame_data: &[u8],
        ) -> Result<(), Self::Error> {
            (*self).send(frame_type, frame_data).await
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
        fn send(&mut self, frame_type: FrameType, frame_data: &[u8]) -> Result<(), Self::Error> {
            self.blocker.block_on(self.api.send(frame_type, frame_data))
        }
    }

    impl<B, R> super::Receiver for Blocking<B, R>
    where
        B: Blocker,
        R: Receiver,
    {
        fn recv(&mut self, frame_data_buf: &mut [u8]) -> Result<(FrameType, usize), Self::Error> {
            self.blocker.block_on(self.api.recv(frame_data_buf))
        }
    }

    impl<E> ErrorType for TrivialUnblocking<E>
    where
        E: ErrorType,
    {
        type Error = E::Error;
    }

    impl<S> Sender for TrivialUnblocking<S>
    where
        S: super::Sender + Send,
    {
        async fn send(
            &mut self,
            frame_type: FrameType,
            frame_data: &[u8],
        ) -> Result<(), Self::Error> {
            self.api.send(frame_type, frame_data)
        }
    }

    impl<R> Receiver for TrivialUnblocking<R>
    where
        R: super::Receiver + Send,
    {
        async fn recv(
            &mut self,
            frame_data_buf: &mut [u8],
        ) -> Result<(FrameType, usize), Self::Error> {
            self.api.recv(frame_data_buf)
        }
    }

    pub mod server {
        pub use super::*;

        pub trait Acceptor: ErrorType {
            type Sender<'a>: Sender<Error = Self::Error>
            where
                Self: 'a;
            type Receiver<'a>: Receiver<Error = Self::Error>
            where
                Self: 'a;

            async fn accept(&self) -> Result<(Self::Sender<'_>, Self::Receiver<'_>), Self::Error>;
        }

        impl<A> Acceptor for &A
        where
            A: Acceptor,
        {
            type Sender<'a> = A::Sender<'a> where Self: 'a;
            type Receiver<'a> = A::Receiver<'a> where Self: 'a;

            async fn accept(&self) -> Result<(Self::Sender<'_>, Self::Receiver<'_>), Self::Error> {
                (*self).accept().await
            }
        }

        impl<A> Acceptor for &mut A
        where
            A: Acceptor,
        {
            type Sender<'a> = A::Sender<'a> where Self: 'a;
            type Receiver<'a> = A::Receiver<'a> where Self: 'a;

            async fn accept(&self) -> Result<(Self::Sender<'_>, Self::Receiver<'_>), Self::Error> {
                (**self).accept().await
            }
        }
    }
}
