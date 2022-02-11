use core::fmt::{Debug, Display};
use core::result::Result;
use core::time::Duration;

use crate::service::Service;

pub trait Spin: Service {
    fn spin(&mut self, duration: Option<Duration>) -> Result<(), Self::Error>;
}

pub trait Postbox<P>: Service {
    fn post(&mut self, payload: &P, wait: Option<Duration>) -> Result<bool, Self::Error>;
}

impl<'a, P, PB> Postbox<P> for &'a mut PB
where
    PB: Postbox<P> + Service,
{
    fn post(&mut self, payload: &P, wait: Option<Duration>) -> Result<bool, Self::Error> {
        (*self).post(payload, wait)
    }
}

pub trait EventBus<P>: Service {
    type Subscription;

    fn subscribe<E>(
        &mut self,
        callback: impl for<'a> FnMut(&'a P) -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Subscription, Self::Error>
    where
        E: Display + Debug + Send + Sync + 'static;
}

impl<'a, P, E> EventBus<P> for &'a mut E
where
    E: EventBus<P>,
{
    type Subscription = E::Subscription;

    fn subscribe<EE>(
        &mut self,
        callback: impl for<'b> FnMut(&'b P) -> Result<(), EE> + Send + 'static,
    ) -> Result<Self::Subscription, Self::Error>
    where
        EE: Display + Debug + Send + Sync + 'static,
    {
        (*self).subscribe(callback)
    }
}

pub trait PostboxProvider<P>: Service {
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

pub trait PinnedEventBus<P>: Service {
    type Subscription;

    fn subscribe<E>(
        &mut self,
        callback: impl for<'a> FnMut(&'a P) -> Result<(), E> + 'static,
    ) -> Result<Self::Subscription, Self::Error>
    where
        E: Display + Debug + Send + Sync + 'static;
}

impl<'a, P, E> PinnedEventBus<P> for &'a mut E
where
    E: PinnedEventBus<P>,
{
    type Subscription = E::Subscription;

    fn subscribe<EE>(
        &mut self,
        callback: impl for<'b> FnMut(&'b P) -> Result<(), EE> + 'static,
    ) -> Result<Self::Subscription, Self::Error>
    where
        EE: Display + Debug + Send + Sync + 'static,
    {
        (*self).subscribe(callback)
    }
}

pub mod nonblocking {
    use crate::channel::nonblocking::{Receiver, Sender};
    use crate::service::Service;

    pub use super::Spin;

    pub trait EventBus<P>: Service {
        type Subscription: Receiver<Data = P, Error = Self::Error>;

        fn subscribe(&mut self) -> Result<Self::Subscription, Self::Error>;
    }

    pub trait PostboxProvider<P>: Service {
        type Postbox: Sender<Data = P, Error = Self::Error>;

        fn postbox(&mut self) -> Result<Self::Postbox, Self::Error>;
    }
}
