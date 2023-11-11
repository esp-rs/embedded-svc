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
    type Timer<'a>: OnceTimer<Error = Self::Error> + PeriodicTimer<Error = Self::Error> + 'a
    where
        Self: 'a;

    fn timer<F>(&self, callback: F) -> Result<Self::Timer<'_>, Self::Error>
    where
        F: FnMut() + Send + 'static;
}

impl<S> TimerService for &S
where
    S: TimerService,
{
    type Timer<'a> = S::Timer<'a> where Self: 'a;

    fn timer<F>(&self, callback: F) -> Result<Self::Timer<'_>, Self::Error>
    where
        F: FnMut() + Send + 'static,
    {
        (*self).timer(callback)
    }
}

impl<S> TimerService for &mut S
where
    S: TimerService,
{
    type Timer<'a> = S::Timer<'a> where Self: 'a;

    fn timer<F>(&self, callback: F) -> Result<Self::Timer<'_>, Self::Error>
    where
        F: FnMut() + Send + 'static,
    {
        (**self).timer(callback)
    }
}

#[cfg(feature = "nightly")]
pub mod asynch {
    use core::time::Duration;

    pub use super::ErrorType;

    #[must_use]
    pub trait OnceTimer: ErrorType {
        async fn after(&mut self, duration: Duration) -> Result<(), Self::Error>;
    }

    impl<O> OnceTimer for &mut O
    where
        O: OnceTimer,
    {
        async fn after(&mut self, duration: Duration) -> Result<(), Self::Error> {
            (*self).after(duration).await
        }
    }

    pub trait Clock {
        async fn tick(&mut self);
    }

    impl<R> Clock for &mut R
    where
        R: Clock,
    {
        async fn tick(&mut self) {
            (*self).tick().await
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
        type Timer<'a>: OnceTimer<Error = Self::Error> + PeriodicTimer<Error = Self::Error> + Send
        where
            Self: 'a;

        async fn timer(&self) -> Result<Self::Timer<'_>, Self::Error>;
    }

    impl<T> TimerService for &T
    where
        T: TimerService,
    {
        type Timer<'a> = T::Timer<'a> where Self: 'a;

        async fn timer(&self) -> Result<Self::Timer<'_>, Self::Error> {
            (*self).timer().await
        }
    }

    impl<T> TimerService for &mut T
    where
        T: TimerService,
    {
        type Timer<'a> = T::Timer<'a> where Self: 'a;

        async fn timer(&self) -> Result<Self::Timer<'_>, Self::Error> {
            (**self).timer().await
        }
    }
}
