use core::fmt::Debug;

pub trait Eth {
    type Error: Debug;

    fn start(&mut self) -> Result<(), Self::Error>;
    fn stop(&mut self) -> Result<(), Self::Error>;

    fn is_started(&self) -> Result<bool, Self::Error>;
    fn is_connected(&self) -> Result<bool, Self::Error>;
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

    fn is_connected(&self) -> Result<bool, Self::Error> {
        (**self).is_connected()
    }
}

pub mod asynch {
    use super::*;

    pub trait Eth {
        type Error: Debug;

        async fn start(&mut self) -> Result<(), Self::Error>;
        async fn stop(&mut self) -> Result<(), Self::Error>;

        async fn is_started(&self) -> Result<bool, Self::Error>;
        async fn is_connected(&self) -> Result<bool, Self::Error>;
    }

    impl<E> Eth for &mut E
    where
        E: Eth,
    {
        type Error = E::Error;

        async fn start(&mut self) -> Result<(), Self::Error> {
            (**self).start().await
        }

        async fn stop(&mut self) -> Result<(), Self::Error> {
            (**self).stop().await
        }

        async fn is_started(&self) -> Result<bool, Self::Error> {
            (**self).is_started().await
        }

        async fn is_connected(&self) -> Result<bool, Self::Error> {
            (**self).is_connected().await
        }
    }
}
