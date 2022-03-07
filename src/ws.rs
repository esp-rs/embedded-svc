use crate::errors::Errors;

pub mod server;

pub type Partial = bool;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FrameType {
    Text(Partial),
    Binary(Partial),
    Ping,
    Pong,
    Close,
    Continue(Partial),
}

impl FrameType {
    pub fn is_partial(&self) -> bool {
        match self {
            Self::Text(partial) | Self::Binary(partial) | Self::Continue(partial) => *partial,
            _ => false,
        }
    }
}

pub trait Receiver: Errors {
    fn recv(&mut self, frame_data_buf: &mut [u8]) -> Result<(FrameType, usize), Self::Error>;
}

pub trait Sender: Errors {
    fn send(&mut self, frame_type: FrameType, frame_data: Option<&[u8]>)
        -> Result<(), Self::Error>;
}

pub mod nonblocking {
    use core::future::Future;

    use crate::errors::Errors;

    pub use super::{FrameType, Partial};

    pub trait Acceptor: Errors {
        type Sender: Sender<Error = Self::Error>;
        type Receiver: Receiver<Error = Self::Error>;

        type AcceptFuture<'a>: Future<
            Output = Result<Option<(Self::Sender, Self::Receiver)>, Self::Error>,
        >
        where
            Self: 'a;

        fn accept(&mut self) -> Result<Self::AcceptFuture<'_>, Self::Error>;
    }

    pub trait Receiver: Errors {
        type ReceiveFuture<'a>: Future<Output = Result<(FrameType, usize), Self::Error>>
        where
            Self: 'a;

        fn recv(
            &mut self,
            frame_data_buf: &mut [u8],
        ) -> Result<Self::ReceiveFuture<'_>, Self::Error>;
    }

    pub trait Sender: Errors {
        type SendFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        fn send(
            &mut self,
            frame_type: FrameType,
            frame_data: Option<&[u8]>,
        ) -> Result<Self::SendFuture<'_>, Self::Error>;
    }
}
