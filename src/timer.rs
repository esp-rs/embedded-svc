use core::fmt::Debug;
use core::result::Result;
use core::time::Duration;

pub trait ErrorType {
    #[cfg(feature = "defmt")]
    type Error: Debug + defmt::Format;
    #[cfg(not(feature = "defmt"))]
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

impl<T> Timer for &mut T
where
    T: Timer,
{
    fn is_scheduled(&self) -> Result<bool, Self::Error> {
        (**self).is_scheduled()
    }

    fn cancel(&mut self) -> Result<bool, Self::Error> {
        (*self).cancel()
    }
}
#[must_use]
pub trait OnceTimer: Timer {
    fn after(&mut self, duration: Duration) -> Result<(), Self::Error>;
}

impl<O> OnceTimer for &mut O
where
    O: OnceTimer,
{
    fn after(&mut self, duration: Duration) -> Result<(), Self::Error> {
        (*self).after(duration)
    }
}

#[must_use]
pub trait PeriodicTimer: Timer {
    fn every(&mut self, duration: Duration) -> Result<(), Self::Error>;
}

impl<P> PeriodicTimer for &mut P
where
    P: PeriodicTimer,
{
    fn every(&mut self, duration: Duration) -> Result<(), Self::Error> {
        (*self).every(duration)
    }
}

pub trait TimerService: ErrorType {
    type Timer: OnceTimer<Error = Self::Error> + PeriodicTimer<Error = Self::Error> + 'static;

    fn timer(
        &mut self,
        callback: impl FnMut() + Send + 'static,
    ) -> Result<Self::Timer, Self::Error>;
}

impl<S> TimerService for &mut S
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

#[cfg(all(feature = "nightly", feature = "experimental"))]
pub mod asynch {
    use core::future::Future;
    use core::time::Duration;

    pub use super::ErrorType;

    #[must_use]
    pub trait OnceTimer: ErrorType {
        type AfterFuture<'a>: Future<Output = ()> + Send
        where
            Self: 'a;

        fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture<'_>, Self::Error>;
    }

    impl<O> OnceTimer for &mut O
    where
        O: OnceTimer,
    {
        type AfterFuture<'a>
        = O::AfterFuture<'a> where Self: 'a;

        fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture<'_>, Self::Error> {
            (*self).after(duration)
        }
    }

    pub trait Clock {
        type TickFuture<'a>: Future<Output = ()> + Send
        where
            Self: 'a;

        fn tick(&mut self) -> Self::TickFuture<'_>;
    }

    impl<R> Clock for &mut R
    where
        R: Clock,
    {
        type TickFuture<'a>
        = R::TickFuture<'a> where Self: 'a;

        fn tick(&mut self) -> Self::TickFuture<'_> {
            (*self).tick()
        }
    }

    #[must_use]
    pub trait PeriodicTimer: ErrorType {
        type Clock<'a>: Clock + Send
        where
            Self: 'a;

        fn every(&mut self, duration: Duration) -> Result<Self::Clock<'_>, Self::Error>;
    }

    impl<P> PeriodicTimer for &mut P
    where
        P: PeriodicTimer,
    {
        type Clock<'a>
        = P::Clock<'a> where Self: 'a;

        fn every(&mut self, duration: Duration) -> Result<Self::Clock<'_>, Self::Error> {
            (*self).every(duration)
        }
    }

    pub trait TimerService: ErrorType {
        type Timer: OnceTimer<Error = Self::Error>
            + PeriodicTimer<Error = Self::Error>
            + Send
            + 'static;

        fn timer(&mut self) -> Result<Self::Timer, Self::Error>;
    }

    impl<T> TimerService for &mut T
    where
        T: TimerService,
    {
        type Timer = T::Timer;

        fn timer(&mut self) -> Result<Self::Timer, Self::Error> {
            (*self).timer()
        }
    }
}
