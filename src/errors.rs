pub use crate::io::Error;
pub use crate::io::ErrorKind;
pub use crate::io::Io as Errors;

#[derive(Debug)]
pub enum EitherError<E1, E2> {
    First(E1),
    Second(E2),
}

impl<E1, E2> core::fmt::Display for EitherError<E2, E1>
where
    E1: core::fmt::Display,
    E2: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::First(e1) => write!(f, "Error {}", e1),
            Self::Second(e2) => write!(f, "Error {}", e2),
        }
    }
}

impl<E1, E2> Error for EitherError<E2, E1>
where
    E1: Error,
    E2: Error,
{
    fn kind(&self) -> ErrorKind {
        match self {
            Self::First(e1) => e1.kind(),
            Self::Second(e2) => e2.kind(),
        }
    }
}

#[cfg(feature = "std")]
impl<E1, E2> std::error::Error for EitherError<E1, E2>
where
    E1: core::fmt::Debug + core::fmt::Display,
    E2: core::fmt::Debug + core::fmt::Display,
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
pub enum EitherError3<E1, E2, E3> {
    First(E1),
    Second(E2),
    Third(E3),
}

impl<E1, E2, E3> core::fmt::Display for EitherError3<E2, E1, E3>
where
    E1: core::fmt::Display,
    E2: core::fmt::Display,
    E3: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::First(e1) => write!(f, "Error {}", e1),
            Self::Second(e2) => write!(f, "Error {}", e2),
            Self::Third(e3) => write!(f, "Error {}", e3),
        }
    }
}

impl<E1, E2, E3> Error for EitherError3<E2, E1, E3>
where
    E1: Error,
    E2: Error,
    E3: Error,
{
    fn kind(&self) -> ErrorKind {
        match self {
            Self::First(e1) => e1.kind(),
            Self::Second(e2) => e2.kind(),
            Self::Third(e3) => e3.kind(),
        }
    }
}

#[cfg(feature = "std")]
impl<E1, E2, E3> std::error::Error for EitherError3<E1, E2, E3>
where
    E1: core::fmt::Debug + core::fmt::Display,
    E2: core::fmt::Debug + core::fmt::Display,
    E3: core::fmt::Debug + core::fmt::Display,
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
pub enum EitherError4<E1, E2, E3, E4> {
    First(E1),
    Second(E2),
    Third(E3),
    Fourth(E4),
}

impl<E1, E2, E3, E4> core::fmt::Display for EitherError4<E2, E1, E3, E4>
where
    E1: core::fmt::Display,
    E2: core::fmt::Display,
    E3: core::fmt::Display,
    E4: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::First(e1) => write!(f, "Error {}", e1),
            Self::Second(e2) => write!(f, "Error {}", e2),
            Self::Third(e3) => write!(f, "Error {}", e3),
            Self::Fourth(e4) => write!(f, "Error {}", e4),
        }
    }
}

impl<E1, E2, E3, E4> Error for EitherError4<E1, E2, E3, E4>
where
    E1: Error,
    E2: Error,
    E3: Error,
    E4: Error,
{
    fn kind(&self) -> ErrorKind {
        match self {
            Self::First(e1) => e1.kind(),
            Self::Second(e2) => e2.kind(),
            Self::Third(e3) => e3.kind(),
            Self::Fourth(e4) => e4.kind(),
        }
    }
}

#[cfg(feature = "std")]
impl<E1, E2, E3, E4> std::error::Error for EitherError4<E1, E2, E3, E4>
where
    E1: core::fmt::Debug + core::fmt::Display,
    E2: core::fmt::Debug + core::fmt::Display,
    E3: core::fmt::Debug + core::fmt::Display,
    E4: core::fmt::Debug + core::fmt::Display,
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
pub enum EitherError5<E1, E2, E3, E4, E5> {
    First(E1),
    Second(E2),
    Third(E3),
    Fourth(E4),
    Fifth(E5),
}

impl<E1, E2, E3, E4, E5> core::fmt::Display for EitherError5<E2, E1, E3, E4, E5>
where
    E1: core::fmt::Display,
    E2: core::fmt::Display,
    E3: core::fmt::Display,
    E4: core::fmt::Display,
    E5: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::First(e1) => write!(f, "Error {}", e1),
            Self::Second(e2) => write!(f, "Error {}", e2),
            Self::Third(e3) => write!(f, "Error {}", e3),
            Self::Fourth(e4) => write!(f, "Error {}", e4),
            Self::Fifth(e5) => write!(f, "Error {}", e5),
        }
    }
}

impl<E1, E2, E3, E4, E5> Error for EitherError5<E1, E2, E3, E4, E5>
where
    E1: Error,
    E2: Error,
    E3: Error,
    E4: Error,
    E5: Error,
{
    fn kind(&self) -> ErrorKind {
        match self {
            Self::First(e1) => e1.kind(),
            Self::Second(e2) => e2.kind(),
            Self::Third(e3) => e3.kind(),
            Self::Fourth(e4) => e4.kind(),
            Self::Fifth(e5) => e5.kind(),
        }
    }
}

#[cfg(feature = "std")]
impl<E1, E2, E3, E4, E5> std::error::Error for EitherError5<E1, E2, E3, E4, E5>
where
    E1: core::fmt::Debug + core::fmt::Display,
    E2: core::fmt::Debug + core::fmt::Display,
    E3: core::fmt::Debug + core::fmt::Display,
    E4: core::fmt::Debug + core::fmt::Display,
    E5: core::fmt::Debug + core::fmt::Display,
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
pub enum EitherError6<E1, E2, E3, E4, E5, E6> {
    First(E1),
    Second(E2),
    Third(E3),
    Fourth(E4),
    Fifth(E5),
    Sixth(E6),
}

impl<E1, E2, E3, E4, E5, E6> core::fmt::Display for EitherError6<E2, E1, E3, E4, E5, E6>
where
    E1: core::fmt::Display,
    E2: core::fmt::Display,
    E3: core::fmt::Display,
    E4: core::fmt::Display,
    E5: core::fmt::Display,
    E6: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::First(e1) => write!(f, "Error {}", e1),
            Self::Second(e2) => write!(f, "Error {}", e2),
            Self::Third(e3) => write!(f, "Error {}", e3),
            Self::Fourth(e4) => write!(f, "Error {}", e4),
            Self::Fifth(e5) => write!(f, "Error {}", e5),
            Self::Sixth(e6) => write!(f, "Error {}", e6),
        }
    }
}

impl<E1, E2, E3, E4, E5, E6> Error for EitherError6<E1, E2, E3, E4, E5, E6>
where
    E1: Error,
    E2: Error,
    E3: Error,
    E4: Error,
    E5: Error,
    E6: Error,
{
    fn kind(&self) -> ErrorKind {
        match self {
            Self::First(e1) => e1.kind(),
            Self::Second(e2) => e2.kind(),
            Self::Third(e3) => e3.kind(),
            Self::Fourth(e4) => e4.kind(),
            Self::Fifth(e5) => e5.kind(),
            Self::Sixth(e6) => e6.kind(),
        }
    }
}

#[cfg(feature = "std")]
impl<E1, E2, E3, E4, E5, E6> std::error::Error for EitherError6<E1, E2, E3, E4, E5, E6>
where
    E1: core::fmt::Debug + core::fmt::Display,
    E2: core::fmt::Debug + core::fmt::Display,
    E3: core::fmt::Debug + core::fmt::Display,
    E4: core::fmt::Debug + core::fmt::Display,
    E5: core::fmt::Debug + core::fmt::Display,
    E6: core::fmt::Debug + core::fmt::Display,
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
pub enum EitherError7<E1, E2, E3, E4, E5, E6, E7> {
    First(E1),
    Second(E2),
    Third(E3),
    Fourth(E4),
    Fifth(E5),
    Sixth(E6),
    Seventh(E7),
}

impl<E1, E2, E3, E4, E5, E6, E7> core::fmt::Display for EitherError7<E2, E1, E3, E4, E5, E6, E7>
where
    E1: core::fmt::Display,
    E2: core::fmt::Display,
    E3: core::fmt::Display,
    E4: core::fmt::Display,
    E5: core::fmt::Display,
    E6: core::fmt::Display,
    E7: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::First(e1) => write!(f, "Error {}", e1),
            Self::Second(e2) => write!(f, "Error {}", e2),
            Self::Third(e3) => write!(f, "Error {}", e3),
            Self::Fourth(e4) => write!(f, "Error {}", e4),
            Self::Fifth(e5) => write!(f, "Error {}", e5),
            Self::Sixth(e6) => write!(f, "Error {}", e6),
            Self::Seventh(e7) => write!(f, "Error {}", e7),
        }
    }
}

impl<E1, E2, E3, E4, E5, E6, E7> Error for EitherError7<E1, E2, E3, E4, E5, E6, E7>
where
    E1: Error,
    E2: Error,
    E3: Error,
    E4: Error,
    E5: Error,
    E6: Error,
    E7: Error,
{
    fn kind(&self) -> ErrorKind {
        match self {
            Self::First(e1) => e1.kind(),
            Self::Second(e2) => e2.kind(),
            Self::Third(e3) => e3.kind(),
            Self::Fourth(e4) => e4.kind(),
            Self::Fifth(e5) => e5.kind(),
            Self::Sixth(e6) => e6.kind(),
            Self::Seventh(e7) => e7.kind(),
        }
    }
}

#[cfg(feature = "std")]
impl<E1, E2, E3, E4, E5, E6, E7> std::error::Error for EitherError7<E1, E2, E3, E4, E5, E6, E7>
where
    E1: core::fmt::Debug + core::fmt::Display,
    E2: core::fmt::Debug + core::fmt::Display,
    E3: core::fmt::Debug + core::fmt::Display,
    E4: core::fmt::Debug + core::fmt::Display,
    E5: core::fmt::Debug + core::fmt::Display,
    E6: core::fmt::Debug + core::fmt::Display,
    E7: core::fmt::Debug + core::fmt::Display,
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

// impl<'e1, S> Errors for &'e1 S
// where
//     S: Errors,
//     <S as embedded_io::Io>::Error: Error,
// {
// }

// impl<'e1, S> Errors for &'e1 mut S
// where
//     S: Errors,
//     <S as embedded_io::Io>::Error: Error,
// {
// }
