use core::fmt::{Debug, Display};
use core::result::Result;
use core::time::Duration;

use crate::service::Service;

pub trait Spin: Service {
    fn spin(&mut self, duration: Option<Duration>) -> Result<(), Self::Error>;
}

pub trait Postbox<P>: Service {
    fn post(&mut self, payload: P, wait: Option<Duration>) -> Result<bool, Self::Error>;
}

pub trait EventBus<P>: Service {
    type Subscription;

    type Postbox: Postbox<P, Error = Self::Error>;

    fn subscribe<E>(
        &mut self,
        callback: impl for<'a> FnMut(&'a P) -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Subscription, Self::Error>
    where
        E: Display + Debug + Send + Sync + 'static;

    fn postbox(&mut self) -> Result<Self::Postbox, Self::Error>;
}

pub trait PinnedEventBus<P>: Service {
    type Subscription;

    type Postbox: Postbox<P, Error = Self::Error>;

    fn subscribe<E>(
        &mut self,
        callback: impl for<'a> FnMut(&'a P) -> Result<(), E> + 'static,
    ) -> Result<Self::Subscription, Self::Error>
    where
        E: Display + Debug + Send + Sync + 'static;

    fn postbox(&mut self) -> Result<Self::Postbox, Self::Error>;
}

pub mod nonblocking {
    use core::fmt::{Debug, Display};

    use crate::channel::nonblocking::{Receiver, Sender};
    use crate::service::Service;

    pub use super::Spin;

    pub trait EventBus<P>: Service {
        type Subscription: Receiver<Data = P, Error = Self::Error>;

        type Postbox: Sender<Data = P, Error = Self::Error>;

        fn subscribe<E>(&mut self) -> Result<Self::Subscription, Self::Error>
        where
            E: Display + Debug + Send + Sync + 'static;

        fn postbox(&mut self) -> Result<Self::Postbox, Self::Error>;
    }
}
