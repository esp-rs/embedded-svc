pub trait Errors {
    #[cfg(not(feature = "std"))]
    type Error: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;
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
