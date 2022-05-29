pub use crate::io::Error;
pub use crate::io::ErrorKind;
pub use crate::io::Io as Errors;

#[derive(Debug)]
pub enum EitherError<A, B> {
    First(A),
    Second(B),
}

impl<A, B> core::fmt::Display for EitherError<B, A>
where
    A: core::fmt::Display,
    B: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EitherError::First(a) => write!(f, "Error {}", a),
            EitherError::Second(b) => write!(f, "Error {}", b),
        }
    }
}

impl<A, B> Error for EitherError<B, A>
where
    A: Error,
    B: Error,
{
    fn kind(&self) -> ErrorKind {
        match self {
            EitherError::First(a) => a.kind(),
            EitherError::Second(b) => b.kind(),
        }
    }
}

#[cfg(feature = "std")]
impl<A, B> std::error::Error for EitherError<A, B>
where
    A: core::fmt::Debug + core::fmt::Display,
    B: core::fmt::Debug + core::fmt::Display,
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

#[derive(Debug)]
pub enum EitherError3<A, B, C> {
    First(A),
    Second(B),
    Third(C),
}

impl<A, B, C> core::fmt::Display for EitherError3<B, A, C>
where
    A: core::fmt::Display,
    B: core::fmt::Display,
    C: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EitherError3::First(a) => write!(f, "Error {}", a),
            EitherError3::Second(b) => write!(f, "Error {}", b),
            EitherError3::Third(c) => write!(f, "Error {}", c),
        }
    }
}

impl<A, B, C> Error for EitherError3<B, A, C>
where
    A: Error,
    B: Error,
    C: Error,
{
    fn kind(&self) -> ErrorKind {
        match self {
            EitherError3::First(a) => a.kind(),
            EitherError3::Second(b) => b.kind(),
            EitherError3::Third(c) => c.kind(),
        }
    }
}

#[cfg(feature = "std")]
impl<A, B, C> std::error::Error for EitherError3<A, B, C>
where
    A: core::fmt::Debug + core::fmt::Display,
    B: core::fmt::Debug + core::fmt::Display,
    C: core::fmt::Debug + core::fmt::Display,
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

#[derive(Debug)]
pub enum EitherError4<A, B, C, D> {
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
}

impl<A, B, C, D> core::fmt::Display for EitherError4<B, A, C, D>
where
    A: core::fmt::Display,
    B: core::fmt::Display,
    C: core::fmt::Display,
    D: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EitherError4::First(a) => write!(f, "Error {}", a),
            EitherError4::Second(b) => write!(f, "Error {}", b),
            EitherError4::Third(c) => write!(f, "Error {}", c),
            EitherError4::Fourth(d) => write!(f, "Error {}", d),
        }
    }
}

impl<A, B, C, D> Error for EitherError4<A, B, C, D>
where
    A: Error,
    B: Error,
    C: Error,
    D: Error,
{
    fn kind(&self) -> ErrorKind {
        match self {
            EitherError4::First(a) => a.kind(),
            EitherError4::Second(b) => b.kind(),
            EitherError4::Third(c) => c.kind(),
            EitherError4::Fourth(d) => d.kind(),
        }
    }
}

#[cfg(feature = "std")]
impl<A, B, C, D> std::error::Error for EitherError4<A, B, C, D>
where
    A: core::fmt::Debug + core::fmt::Display,
    B: core::fmt::Debug + core::fmt::Display,
    C: core::fmt::Debug + core::fmt::Display,
    D: core::fmt::Debug + core::fmt::Display,
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

// pub trait Errors: Io where <Self as Io>::Error: Error {
// }

// impl<'a, S> Errors for &'a S
// where
//     S: Errors,
//     <S as embedded_io::Io>::Error: Error,
// {
// }

// impl<'a, S> Errors for &'a mut S
// where
//     S: Errors,
//     <S as embedded_io::Io>::Error: Error,
// {
// }
