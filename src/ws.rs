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
        type Connection<'a>: Sender<Error = Self::Error> + Receiver<Error = Self::Error>
        where
            Self: 'a;

        fn accept(&self) -> Result<Self::Connection<'_>, Self::Error>;
    }

    impl<A> Acceptor for &A
    where
        A: Acceptor,
    {
        type Connection<'a> = A::Connection<'a> where Self: 'a;

        fn accept(&self) -> Result<Self::Connection<'_>, Self::Error> {
            (*self).accept()
        }
    }

    impl<A> Acceptor for &mut A
    where
        A: Acceptor,
    {
        type Connection<'a> = A::Connection<'a> where Self: 'a;

        fn accept(&self) -> Result<Self::Connection<'_>, Self::Error> {
            (**self).accept()
        }
    }
}

pub mod asynch {
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
