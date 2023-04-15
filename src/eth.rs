use core::fmt::Debug;

pub trait Eth {
    type Error: Debug;

    fn start(&mut self) -> Result<(), Self::Error>;
    fn stop(&mut self) -> Result<(), Self::Error>;

    fn is_started(&self) -> Result<bool, Self::Error>;
    fn is_up(&self) -> Result<bool, Self::Error>;
}

impl<E> Eth for &mut E
where
    E: Eth,
{
    type Error = E::Error;

    fn start(&mut self) -> Result<(), Self::Error> {
        (*self).start()
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        (*self).stop()
    }

    fn is_started(&self) -> Result<bool, Self::Error> {
        (**self).is_started()
    }

    fn is_up(&self) -> Result<bool, Self::Error> {
        (**self).is_up()
    }
}

#[cfg(all(feature = "nightly", feature = "experimental"))]
pub mod asynch {
    use futures::Future;

    use super::*;

    pub trait Eth {
        type Error: Debug;

        type StartFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type StopFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type IsStartedFuture<'a>: Future<Output = Result<bool, Self::Error>>
        where
            Self: 'a;

        type IsUpFuture<'a>: Future<Output = Result<bool, Self::Error>>
        where
            Self: 'a;

        fn start(&mut self) -> Self::StartFuture<'_>;
        fn stop(&mut self) -> Self::StopFuture<'_>;

        fn is_started(&self) -> Self::IsStartedFuture<'_>;
        fn is_up(&self) -> Self::IsUpFuture<'_>;
    }

    impl<E> Eth for &mut E
    where
        E: Eth,
    {
        type Error = E::Error;

        type StartFuture<'a> = E::StartFuture<'a>
        where
            Self: 'a;

        type StopFuture<'a> = E::StopFuture<'a>
        where
            Self: 'a;

        type IsStartedFuture<'a> = E::IsStartedFuture<'a>
        where
            Self: 'a;

        type IsUpFuture<'a> = E::IsUpFuture<'a>
        where
            Self: 'a;

        fn start(&mut self) -> Self::StartFuture<'_> {
            (**self).start()
        }

        fn stop(&mut self) -> Self::StopFuture<'_> {
            (**self).stop()
        }

        fn is_started(&self) -> Self::IsStartedFuture<'_> {
            (**self).is_started()
        }

        fn is_up(&self) -> Self::IsUpFuture<'_> {
            (**self).is_up()
        }
    }
}
