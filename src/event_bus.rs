use core::result::Result;
use core::time::Duration;

use crate::errors::Errors;

pub trait Spin: Errors {
    fn spin(&mut self, duration: Option<Duration>) -> Result<(), Self::Error>;
}

pub trait Postbox<P>: Errors {
    fn post(&mut self, payload: &P, wait: Option<Duration>) -> Result<bool, Self::Error>;
}

impl<'a, P, PB> Postbox<P> for &'a mut PB
where
    PB: Postbox<P> + Errors,
{
    fn post(&mut self, payload: &P, wait: Option<Duration>) -> Result<bool, Self::Error> {
        (*self).post(payload, wait)
    }
}

pub trait EventBus<P>: Errors {
    type Subscription;

    fn subscribe(
        &mut self,
        callback: impl for<'a> FnMut(&'a P) + Send + 'static,
    ) -> Result<Self::Subscription, Self::Error>;
}

impl<'a, P, E> EventBus<P> for &'a mut E
where
    E: EventBus<P>,
{
    type Subscription = E::Subscription;

    fn subscribe(
        &mut self,
        callback: impl for<'b> FnMut(&'b P) + Send + 'static,
    ) -> Result<Self::Subscription, Self::Error> {
        (*self).subscribe(callback)
    }
}

pub trait PostboxProvider<P>: Errors {
    type Postbox: Postbox<P, Error = Self::Error>;

    fn postbox(&mut self) -> Result<Self::Postbox, Self::Error>;
}

impl<'a, P, PP> PostboxProvider<P> for &'a mut PP
where
    PP: PostboxProvider<P>,
{
    type Postbox = PP::Postbox;

    fn postbox(&mut self) -> Result<Self::Postbox, Self::Error> {
        (*self).postbox()
    }
}

pub trait PinnedEventBus<P>: Errors {
    type Subscription;

    fn subscribe(
        &mut self,
        callback: impl for<'a> FnMut(&'a P) + 'static,
    ) -> Result<Self::Subscription, Self::Error>;
}

impl<'a, P, E> PinnedEventBus<P> for &'a mut E
where
    E: PinnedEventBus<P>,
{
    type Subscription = E::Subscription;

    fn subscribe(
        &mut self,
        callback: impl for<'b> FnMut(&'b P) + 'static,
    ) -> Result<Self::Subscription, Self::Error> {
        (*self).subscribe(callback)
    }
}

#[cfg(feature = "experimental")]
pub mod asyncs {
    use crate::channel::asyncs::{Receiver, Sender};
    use crate::errors::Errors;

    pub use super::Spin;

    pub trait EventBus<P>: Errors {
        type Subscription: Receiver<Data = P>;

        fn subscribe(&mut self) -> Result<Self::Subscription, Self::Error>;
    }

    pub trait PostboxProvider<P>: Errors {
        type Postbox: Sender<Data = P>;

        fn postbox(&mut self) -> Result<Self::Postbox, Self::Error>;
    }
}
