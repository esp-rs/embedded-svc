pub trait Service {
    #[cfg(not(feature = "std"))]
    type Error: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;
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
