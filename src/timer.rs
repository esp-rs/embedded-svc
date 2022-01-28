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
        &mut self,
        duration: Duration,
        callback: impl FnOnce() -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}

pub trait Periodic: Service {
    type Timer: Timer<Error = Self::Error> + 'static;

    fn every<E>(
        &mut self,
        duration: Duration,
        callback: impl FnMut() -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}

pub trait PinnedOnce: Service {
    type Timer: Timer<Error = Self::Error> + 'static;

    fn after<E>(
        &mut self,
        duration: Duration,
        callback: impl FnOnce() -> Result<(), E> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}

pub trait PinnedPeriodic: Service {
    type Timer: Timer<Error = Self::Error> + 'static;

    fn every<E>(
        &mut self,
        duration: Duration,
        callback: impl FnMut() -> Result<(), E> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}

pub mod nonblocking {
    use core::future::Future;
    use core::time::Duration;

    use crate::service::Service;

    pub trait Once: Service {
        type AfterFuture: Future<Output = Result<(), Self::Error>>;

        fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture, Self::Error>;
    }

    /// core.stream.Stream is not stable yet. Therefore, we have to use a Future instead
    pub trait Timer: Service {
        type NextFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        fn next(&mut self) -> Self::NextFuture<'_>;
    }

    pub trait Periodic: Service {
        type Timer: Timer;

        fn every(&mut self, duration: Duration) -> Result<Self::Timer, Self::Error>;
    }
}
