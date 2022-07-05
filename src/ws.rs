use core::fmt::Debug;

pub mod server;

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

pub trait Acceptor: ErrorType {
    // TODO:
    // - We probably need to parameterize Sender & Receiver by <'a>
    // - Consequently, we need to change `accept(&mut self)` to `accept(&self)`

    type Sender: Sender<Error = Self::Error> + Send;
    type Receiver: Receiver<Error = Self::Error> + Send;

    fn accept(&mut self) -> Result<Option<(Self::Sender, Self::Receiver)>, Self::Error>;
}

impl<A> Acceptor for &mut A
where
    A: Acceptor,
{
    type Sender = A::Sender;

    type Receiver = A::Receiver;

    fn accept(&mut self) -> Result<Option<(Self::Sender, Self::Receiver)>, Self::Error> {
        (*self).accept()
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

pub mod asynch {
    use core::future::Future;

    use crate::unblocker::asynch::{Blocker, Blocking};

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

    impl<A> Acceptor for &mut A
    where
        A: Acceptor,
    {
        // TODO:
        // - We probably need to parameterize Sender & Receiver by <'a>
        // - Consequently, we need to change `accept(&mut self)` to `accept(&self)`

        type Sender = A::Sender;

        type Receiver = A::Receiver;

        type AcceptFuture<'a>
        where
            Self: 'a,
        = A::AcceptFuture<'a>;

        fn accept(&mut self) -> Self::AcceptFuture<'_> {
            (*self).accept()
        }
    }

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

    impl<B, A> super::Acceptor for Blocking<B, A>
    where
        B: Blocker + Clone + Send,
        A: Acceptor,
    {
        type Sender = Blocking<B, A::Sender>;

        type Receiver = Blocking<B, A::Receiver>;

        fn accept(&mut self) -> Result<Option<(Self::Sender, Self::Receiver)>, Self::Error> {
            let r = self.0.block_on(self.1.accept())?;

            Ok(r.map(|(sender, receiver)| {
                (
                    Blocking::new(self.0.clone(), sender),
                    Blocking::new(self.0.clone(), receiver),
                )
            }))
        }
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
}
