use core::fmt;

use crate::io::Io;
use crate::ws::server::*;

pub trait Registry: Io {
    type Receiver: Receiver<Error = Self::Error> + SessionProvider;
    type Sender: Sender<Error = Self::Error> + SenderFactory<Error = Self::Error>;

    fn ws<'a>(&'a mut self, uri: &'a str) -> HandlerRegistrationBuilder<'a, Self>
    where
        Self: Sized,
    {
        HandlerRegistrationBuilder {
            uri,
            registry: self,
        }
    }

    fn set_handler<H, E>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(&'a mut Self::Receiver, &'a mut Self::Sender) -> Result<(), E> + 'static,
        E: fmt::Debug;
}

pub struct HandlerRegistrationBuilder<'r, R> {
    uri: &'r str,
    registry: &'r mut R,
}

impl<'r, R> HandlerRegistrationBuilder<'r, R>
where
    R: Registry,
{
    pub fn handler<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(&'a mut R::Receiver, &'a mut R::Sender) -> Result<(), E> + 'static,
        E: fmt::Debug,
    {
        self.registry.set_handler(self.uri, handler)
    }
}
