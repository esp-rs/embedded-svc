#[cfg(feature = "std")]
pub trait Error: std::error::Error + Send + Sync + 'static {}

#[cfg(not(feature = "std"))]
pub trait Error: Debug + Display + Send + Sync + 'static {}

#[cfg(not(feature = "std"))]
impl<E> Error for E where E: Debug + Display + Send + Sync + 'static {}

#[cfg(feature = "std")]
impl<E> Error for E where E: std::error::Error + Send + Sync + 'static {}

pub trait Errors {
    type Error: Error;
}

impl<'a, S> Errors for &'a S
where
    S: Errors,
{
    type Error = S::Error;
}

impl<'a, S> Errors for &'a mut S
where
    S: Errors,
{
    type Error = S::Error;
}
