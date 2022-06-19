use core::fmt::Debug;

use crate::ws::*;

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

pub mod registry {
    use core::fmt::Debug;

    use super::{Receiver, Sender, SenderFactory, SessionProvider};

    pub trait Registry {
        type Error: Debug;

        type SendReceiveError: Debug;

        type Receiver: Receiver<Error = Self::SendReceiveError> + SessionProvider;
        type Sender: Sender<Error = Self::SendReceiveError>
            + SenderFactory<Error = Self::SendReceiveError>;

        fn handle_ws<H, E>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
        where
            H: for<'a> Fn(&'a mut Self::Receiver, &'a mut Self::Sender) -> Result<(), E>
                + Send
                + 'static,
            E: Debug;
    }
}
