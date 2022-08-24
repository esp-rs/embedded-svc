use core::fmt::Debug;

pub trait Eth {
    type Error: Debug;

    fn start(&mut self) -> Result<(), Self::Error>;
    fn stop(&mut self) -> Result<(), Self::Error>;

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

    fn is_up(&self) -> Result<bool, Self::Error> {
        (**self).is_up()
    }
}
