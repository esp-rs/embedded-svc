use core::fmt::{Debug, Display};
use core::result::Result;
use core::time::Duration;

use crate::service::Service;

pub trait Postbox: Service {
    type Data: Copy + Send;

    fn post(&self, payload: Self::Data) -> Result<(), Self::Error>;
}

pub trait Spin: Service {
    fn spin(&self, duration: Option<Duration>) -> Result<(), Self::Error>;
}

pub trait EventBus: Service {
    type Data: Copy + Send;

    type Subscription: Send;

    type Postbox: Postbox<Data = Self::Data>;

    fn subscribe<E>(
        &self,
        callback: impl for<'a> FnMut(&'a Self::Data) -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Subscription, Self::Error>
    where
        E: Display + Debug + Send + Sync + 'static;

    fn postbox(&self) -> Result<Self::Postbox, Self::Error>;
}

pub trait PinnedEventBus: Service {
    type Data: Copy + Send;

    type Subscription;

    type Postbox: Postbox<Data = Self::Data>;

    fn subscribe<E>(
        &self,
        callback: impl for<'a> FnMut(&'a Self::Data) -> Result<(), E> + 'static,
    ) -> Result<Self::Subscription, Self::Error>
    where
        E: Display + Debug + Send + Sync + 'static;

    fn postbox(&self) -> Result<Self::Postbox, Self::Error>;
}

pub mod nonblocking {
    use core::fmt::{Debug, Display};
    use core::stream::Stream;

    use crate::service::Service;

    pub use super::Postbox;

    pub trait EventBus: Service {
        type Data: Copy + Send;

        type SubscriptionStream: Stream<Item = Result<Self::Data, Self::Error>>;

        type Postbox: Postbox<Data = Self::Data>;

        fn subscribe<E>(&self) -> Result<Self::SubscriptionStream, Self::Error>
        where
            E: Display + Debug + Send + Sync + 'static;

        fn postbox(&self) -> Result<Self::Postbox, Self::Error>;
    }
}
