use core::fmt::{Debug, Display};

pub trait Service {
    type Error: Display + Debug + Send + Sync + 'static;
}

impl<'a, S> Service for &'a S
where
    S: Service,
{
    type Error = S::Error;
}

impl<'a, S> Service for &'a mut S
where
    S: Service,
{
    type Error = S::Error;
}
