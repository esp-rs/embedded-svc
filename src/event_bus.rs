use core::fmt::{Debug, Display};
use core::result::Result;
use core::time::Duration;

use crate::service::Service;

pub trait Spin: Service {
    fn spin(&mut self, duration: Option<Duration>) -> Result<(), Self::Error>;
}

pub trait Postbox<P>: Service {
    fn post(&mut self, payload: P) -> Result<(), Self::Error>;
}

pub trait EventBus<P>: Service {
    type Subscription;

    type Postbox: Postbox<P>;

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

    type Postbox: Postbox<P>;

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
    use core::stream::Stream;

    use crate::service::Service;

    pub use super::Postbox;
    pub use super::Spin;

    pub trait EventBus<P>: Service {
        type SubscriptionStream: Stream<Item = Result<P, Self::Error>>;

        type Postbox: Postbox<P>;

        fn subscribe<E>(&mut self) -> Result<Self::SubscriptionStream, Self::Error>
        where
            E: Display + Debug + Send + Sync + 'static;

        fn postbox(&mut self) -> Result<Self::Postbox, Self::Error>;
    }
}
