use core::fmt::Debug;

use crate::errors::*;
use crate::ws::*;

pub mod registry;

pub trait SessionProvider {
    type Session: Clone + Send + PartialEq + Debug;

    fn session(&self) -> Self::Session;

    fn is_closed(&self) -> bool;
}

pub trait SenderFactory: Errors {
    type Sender: Sender;

    fn create(&self) -> Result<Self::Sender, Self::Error>;
}
