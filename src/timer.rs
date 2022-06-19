use core::fmt::Debug;
use core::result::Result;
use core::time::Duration;

pub trait ErrorType {
    type Error: Debug;
}

impl<E> ErrorType for &E
where
    E: ErrorType,
{
    type Error = E::Error;
}

impl<E> ErrorType for &mut E
where
    E: ErrorType,
{
    type Error = E::Error;
}

#[must_use]
pub trait Timer: ErrorType {
    fn is_scheduled(&self) -> Result<bool, Self::Error>;

    fn cancel(&mut self) -> Result<bool, Self::Error>;
}

#[must_use]
pub trait OnceTimer: Timer {
    fn after(&mut self, duration: Duration) -> Result<(), Self::Error>;
}

#[must_use]
pub trait PeriodicTimer: Timer {
    fn every(&mut self, duration: Duration) -> Result<(), Self::Error>;
}

pub trait TimerService: ErrorType {
    type Timer: OnceTimer<Error = Self::Error> + PeriodicTimer<Error = Self::Error> + 'static;

    fn timer(
        &mut self,
        callback: impl FnMut() + Send + 'static,
    ) -> Result<Self::Timer, Self::Error>;
}

impl<'a, S> TimerService for &'a mut S
where
    S: TimerService,
{
    type Timer = S::Timer;

    fn timer(
        &mut self,
        callback: impl FnMut() + Send + 'static,
    ) -> Result<Self::Timer, Self::Error> {
        (*self).timer(callback)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;
    use core::time::Duration;

    use crate::channel::asynch::Receiver;

    pub use super::ErrorType;

    #[must_use]
    pub trait OnceTimer: ErrorType {
        type AfterFuture<'a>: Future<Output = ()> + Send
        where
            Self: 'a;

        fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture<'_>, Self::Error>;
    }

    #[must_use]
    pub trait PeriodicTimer: ErrorType {
        type Clock<'a>: Receiver<Data = ()> + Send
        where
            Self: 'a;

        fn every(&mut self, duration: Duration) -> Result<Self::Clock<'_>, Self::Error>;
    }

    pub trait TimerService: ErrorType {
        type Timer: OnceTimer<Error = Self::Error>
            + PeriodicTimer<Error = Self::Error>
            + Send
            + 'static;

        fn timer(&mut self) -> Result<Self::Timer, Self::Error>;
    }
}
