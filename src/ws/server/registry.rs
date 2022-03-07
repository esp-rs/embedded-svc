use core::fmt;

extern crate alloc;
use alloc::string::{String, ToString};

use crate::errors::Errors;

use crate::ws::server::*;
use crate::ws::*;

pub trait Registry: Errors {
    type Receiver: Receiver + SessionProvider;
    type Sender: Sender + SenderFactory;

    fn at(&mut self, uri: impl ToString) -> HandlerRegistrationBuilder<Self>
    where
        Self: Sized,
    {
        HandlerRegistrationBuilder {
            uri: uri.to_string(),
            registry: self,
        }
    }

    fn set_handler<H, E>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(&'a mut Self::Receiver, &'a mut Self::Sender) -> Result<(), E> + 'static,
        E: fmt::Display + fmt::Debug;
}

pub struct HandlerRegistrationBuilder<'r, R> {
    uri: String,
    registry: &'r mut R,
}

impl<'r, R> HandlerRegistrationBuilder<'r, R>
where
    R: Registry,
{
    pub fn handler<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(&'a mut R::Receiver, &'a mut R::Sender) -> Result<(), E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.registry.set_handler(self.uri.as_str(), handler)
    }
}
