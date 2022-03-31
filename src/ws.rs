use crate::errors::Errors;

pub mod server;

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

pub trait Receiver: Errors {
    fn recv(&mut self, frame_data_buf: &mut [u8]) -> Result<(FrameType, usize), Self::Error>;
}

pub trait Sender: Errors {
    fn send(&mut self, frame_type: FrameType, frame_data: Option<&[u8]>)
        -> Result<(), Self::Error>;
}

pub mod asyncs {
    use core::future::Future;

    use crate::errors::Errors;

    pub use super::{Fragmented, FrameType};

    pub trait Acceptor: Errors {
        type Sender: Sender<Error = Self::Error>;
        type Receiver: Receiver<Error = Self::Error>;

        type AcceptFuture<'a>: Future<
            Output = Result<Option<(Self::Sender, Self::Receiver)>, Self::Error>,
        >
        where
            Self: 'a;

        fn accept(&mut self) -> Self::AcceptFuture<'_>;
    }

    pub trait Receiver: Errors {
        type ReceiveFuture<'a>: Future<Output = Result<(FrameType, usize), Self::Error>>
        where
            Self: 'a;

        fn recv<'a>(&'a mut self, frame_data_buf: &'a mut [u8]) -> Self::ReceiveFuture<'a>;
    }

    pub trait Sender: Errors {
        type SendFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        fn send<'a>(
            &'a mut self,
            frame_type: FrameType,
            frame_data: Option<&'a [u8]>,
        ) -> Self::SendFuture<'a>;
    }
}
