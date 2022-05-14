use core::fmt;
#[cfg(not(feature = "std"))]
use core::fmt::{Debug, Display};

#[cfg(feature = "std")]
pub trait Error: std::error::Error + Send + Sync + 'static {}

#[cfg(not(feature = "std"))]
pub trait Error: Debug + Display + Send + Sync + 'static {}

#[cfg(not(feature = "std"))]
impl<E> Error for E where E: Debug + Display + Send + Sync + 'static {}

#[cfg(feature = "std")]
impl<E> Error for E where E: std::error::Error + Send + Sync + 'static {}

#[derive(Debug)]
pub enum EitherError<A, B>
where
    A: Error,
    B: Error,
{
    First(A),
    Second(B),
}

impl<A, B> fmt::Display for EitherError<B, A>
where
    A: Error,
    B: Error,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EitherError::First(a) => write!(f, "Error {}", a),
            EitherError::Second(b) => write!(f, "Error {}", b),
        }
    }
}

#[cfg(feature = "std")]
impl<A, B> std::error::Error for EitherError<A, B>
where
    A: Error,
    B: Error,
    // TODO
    // where
    //     R: std::error::Error + 'static,
    //     W: std::error::Error + 'static,
{
    // TODO
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     match self {
    //         CopyError::ReadError(r) => Some(r),
    //         CopyError::WriteError(w) => Some(w),
    //     }
    // }
}

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
