use core::fmt::Debug;

pub mod server;

pub trait ErrorType {
    type Error: Debug;
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

pub trait Sender: ErrorType {
    fn send(&mut self, frame_type: FrameType, frame_data: Option<&[u8]>)
        -> Result<(), Self::Error>;
}

pub mod asynch {
    use core::future::Future;

    pub use super::{ErrorType, Fragmented, FrameType};

    pub trait Acceptor: ErrorType {
        type Sender: Sender<Error = Self::Error> + Send;
        type Receiver: Receiver<Error = Self::Error> + Send;

        type AcceptFuture<'a>: Future<Output = Result<Option<(Self::Sender, Self::Receiver)>, Self::Error>>
            + Send
        where
            Self: 'a;

        fn accept(&mut self) -> Self::AcceptFuture<'_>;
    }

    pub trait Receiver: ErrorType {
        type ReceiveFuture<'a>: Future<Output = Result<(FrameType, usize), Self::Error>> + Send
        where
            Self: 'a;

        fn recv<'a>(&'a mut self, frame_data_buf: &'a mut [u8]) -> Self::ReceiveFuture<'a>;
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
}
