use core::fmt::{Debug, Display};
use core::result::Result;
use core::time::Duration;

use crate::service::Service;

#[must_use]
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

impl<'a, O> Once for &'a mut O
where
    O: Once,
{
    type Timer = O::Timer;

    fn after<E>(
        &mut self,
        duration: Duration,
        callback: impl FnOnce() -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static,
    {
        (*self).after(duration, callback)
    }
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

impl<'a, P> Periodic for &'a mut P
where
    P: Periodic,
{
    type Timer = P::Timer;

    fn every<E>(
        &mut self,
        duration: Duration,
        callback: impl FnMut() -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static,
    {
        (*self).every(duration, callback)
    }
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

impl<'a, O> PinnedOnce for &'a mut O
where
    O: PinnedOnce,
{
    type Timer = O::Timer;

    fn after<E>(
        &mut self,
        duration: Duration,
        callback: impl FnOnce() -> Result<(), E> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static,
    {
        (*self).after(duration, callback)
    }
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

impl<'a, P> PinnedPeriodic for &'a mut P
where
    P: PinnedPeriodic,
{
    type Timer = P::Timer;

    fn every<E>(
        &mut self,
        duration: Duration,
        callback: impl FnMut() -> Result<(), E> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static,
    {
        (*self).every(duration, callback)
    }
}

pub mod nonblocking {
    use core::future::Future;
    use core::time::Duration;

    use crate::channel::nonblocking::Receiver;
    use crate::service::Service;

    pub trait Once: Service {
        type AfterFuture: Future<Output = Result<(), Self::Error>>;

        fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture, Self::Error>;
    }

    pub trait Periodic: Service {
        type Timer: Receiver<Data = (), Error = Self::Error>;

        fn every(&mut self, duration: Duration) -> Result<Self::Timer, Self::Error>;
    }
}
