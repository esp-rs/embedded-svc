use core::fmt::Debug;

pub trait Eth {
    #[cfg(feature = "defmt")]
    type Error: Debug + defmt::Format;
    #[cfg(not(feature = "defmt"))]
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
