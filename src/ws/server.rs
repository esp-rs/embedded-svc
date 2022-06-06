use core::fmt::Debug;

use crate::io::Io;
use crate::ws::*;

pub mod registry;

pub trait SessionProvider {
    type Session: Clone + Send + PartialEq + Debug;

    fn session(&self) -> Self::Session;

    fn is_new(&self) -> bool;
    fn is_closed(&self) -> bool;
}

pub trait SenderFactory: Io {
    type Sender: Sender<Error = Self::Error>;

    fn create(&self) -> Result<Self::Sender, Self::Error>;
}
