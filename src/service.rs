use core::fmt::Debug;

pub trait Service {
    type Error: Debug;
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
