use core::fmt::{Debug, Display};
use core::result::Result;
use core::time::Duration;

use crate::service::Service;

pub trait Timer: Service {
    fn start(&mut self) -> Result<(), Self::Error>;

    fn is_scheduled(&self) -> Result<bool, Self::Error>;

    fn cancel(&mut self) -> Result<bool, Self::Error>;
}

pub trait Once: Service {
    type Timer: Timer<Error = Self::Error> + 'static;

    fn after<E>(
        &self,
        duration: Duration,
        callback: impl FnOnce() -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}

pub trait Periodic: Service {
    type Timer: Timer<Error = Self::Error> + 'static;

    fn every<E>(
        &self,
        duration: Duration,
        callback: impl FnMut() -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}

pub trait PinnedOnce: Service {
    type Timer: Timer<Error = Self::Error> + 'static;

    fn after<E>(
        &self,
        duration: Duration,
        callback: impl FnOnce() -> Result<(), E> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}

pub trait PinnedPeriodic: Service {
    type Timer: Timer<Error = Self::Error> + 'static;

    fn every<E>(
        &self,
        duration: Duration,
        callback: impl FnMut() -> Result<(), E> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}

pub mod nonblocking {
    use core::future::Future;
    use core::stream::Stream;
    use core::time::Duration;

    use crate::service::Service;

    pub trait Once: Service {
        type AfterFuture: Future<Output = Result<(), Self::Error>>;

        fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture, Self::Error>;
    }

    pub trait Periodic: Service {
        type EveryStream: Stream<Item = Result<(), Self::Error>>;

        fn every(&mut self, duration: Duration) -> Result<Self::EveryStream, Self::Error>;
    }
}
