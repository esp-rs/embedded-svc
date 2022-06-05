pub use crate::io::Error;
pub use crate::io::ErrorKind;
pub use crate::io::Io as Errors;

pub mod conv {
    use core::fmt::{self, Display, Formatter};

    use super::{Error, ErrorKind};

    #[derive(Debug)]
    pub struct StrConvError;

    impl Display for StrConvError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "StrConvError")
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for StrConvError {}

    impl Error for StrConvError {
        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }
}

pub mod wrap {
    use core::fmt::{self, Debug, Display, Formatter};

    use super::{Error, ErrorKind};

    #[derive(Debug)]
    pub struct WrapError<E>(pub E);

    impl<E> From<E> for WrapError<E>
    where
        E: Debug,
    {
        fn from(e: E) -> Self {
            WrapError(e)
        }
    }

    impl<E> Display for WrapError<E>
    where
        E: Display,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl<E> Error for WrapError<E>
    where
        E: Error,
    {
        fn kind(&self) -> ErrorKind {
            self.0.kind()
        }
    }

    #[cfg(feature = "std")]
    impl<E> std::error::Error for WrapError<E> where E: Display + Debug {}

    #[derive(Debug)]
    pub enum EitherError<E1, E2> {
        E1(E1),
        E2(E2),
    }

    impl<E1, E2> Display for EitherError<E2, E1>
    where
        E1: Display,
        E2: Display,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                Self::E1(e) => write!(f, "E1: {}", e),
                Self::E2(e) => write!(f, "E2: {}", e),
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
                Self::E1(e) => e.kind(),
                Self::E2(e) => e.kind(),
            }
        }
    }

    #[cfg(feature = "std")]
    impl<E1, E2> std::error::Error for EitherError<E1, E2>
    where
        E1: Display + Debug,
        E2: Display + Debug,
    {
    }

    #[derive(Debug)]
    pub enum EitherError3<E1, E2, E3> {
        E1(E1),
        E2(E2),
        E3(E3),
    }

    impl<E1, E2, E3> Display for EitherError3<E2, E1, E3>
    where
        E1: Display,
        E2: Display,
        E3: Display,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                Self::E1(e) => write!(f, "E1: {}", e),
                Self::E2(e) => write!(f, "E2: {}", e),
                Self::E3(e) => write!(f, "E3: {}", e),
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
                Self::E1(e) => e.kind(),
                Self::E2(e) => e.kind(),
                Self::E3(e) => e.kind(),
            }
        }
    }

    #[cfg(feature = "std")]
    impl<E1, E2, E3> std::error::Error for EitherError3<E1, E2, E3>
    where
        E1: Debug + Display,
        E2: Debug + Display,
        E3: Debug + Display,
    {
    }

    #[derive(Debug)]
    pub enum EitherError4<E1, E2, E3, E4> {
        E1(E1),
        E2(E2),
        E3(E3),
        E4(E4),
    }

    impl<E1, E2, E3, E4> Display for EitherError4<E2, E1, E3, E4>
    where
        E1: Display,
        E2: Display,
        E3: Display,
        E4: Display,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                Self::E1(e) => write!(f, "Error {}", e),
                Self::E2(e) => write!(f, "Error {}", e),
                Self::E3(e) => write!(f, "Error {}", e),
                Self::E4(e) => write!(f, "Error {}", e),
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
                Self::E1(e) => e.kind(),
                Self::E2(e) => e.kind(),
                Self::E3(e) => e.kind(),
                Self::E4(e) => e.kind(),
            }
        }
    }

    #[cfg(feature = "std")]
    impl<E1, E2, E3, E4> std::error::Error for EitherError4<E1, E2, E3, E4>
    where
        E1: Debug + Display,
        E2: Debug + Display,
        E3: Debug + Display,
        E4: Debug + Display,
    {
    }

    #[derive(Debug)]
    pub enum EitherError5<E1, E2, E3, E4, E5> {
        E1(E1),
        E2(E2),
        E3(E3),
        E4(E4),
        E5(E5),
    }

    impl<E1, E2, E3, E4, E5> Display for EitherError5<E2, E1, E3, E4, E5>
    where
        E1: Display,
        E2: Display,
        E3: Display,
        E4: Display,
        E5: Display,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                Self::E1(e) => write!(f, "Error {}", e),
                Self::E2(e) => write!(f, "Error {}", e),
                Self::E3(e) => write!(f, "Error {}", e),
                Self::E4(e) => write!(f, "Error {}", e),
                Self::E5(e) => write!(f, "Error {}", e),
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
                Self::E1(e) => e.kind(),
                Self::E2(e) => e.kind(),
                Self::E3(e) => e.kind(),
                Self::E4(e) => e.kind(),
                Self::E5(e) => e.kind(),
            }
        }
    }

    #[cfg(feature = "std")]
    impl<E1, E2, E3, E4, E5> std::error::Error for EitherError5<E1, E2, E3, E4, E5>
    where
        E1: Debug + Display,
        E2: Debug + Display,
        E3: Debug + Display,
        E4: Debug + Display,
        E5: Debug + Display,
    {
    }

    #[derive(Debug)]
    pub enum EitherError6<E1, E2, E3, E4, E5, E6> {
        E1(E1),
        E2(E2),
        E3(E3),
        E4(E4),
        E5(E5),
        E6(E6),
    }

    impl<E1, E2, E3, E4, E5, E6> Display for EitherError6<E2, E1, E3, E4, E5, E6>
    where
        E1: Display,
        E2: Display,
        E3: Display,
        E4: Display,
        E5: Display,
        E6: Display,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                Self::E1(e) => write!(f, "Error {}", e),
                Self::E2(e) => write!(f, "Error {}", e),
                Self::E3(e) => write!(f, "Error {}", e),
                Self::E4(e) => write!(f, "Error {}", e),
                Self::E5(e) => write!(f, "Error {}", e),
                Self::E6(e) => write!(f, "Error {}", e),
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
                Self::E1(e) => e.kind(),
                Self::E2(e) => e.kind(),
                Self::E3(e) => e.kind(),
                Self::E4(e) => e.kind(),
                Self::E5(e) => e.kind(),
                Self::E6(e) => e.kind(),
            }
        }
    }

    #[cfg(feature = "std")]
    impl<E1, E2, E3, E4, E5, E6> std::error::Error for EitherError6<E1, E2, E3, E4, E5, E6>
    where
        E1: Debug + Display,
        E2: Debug + Display,
        E3: Debug + Display,
        E4: Debug + Display,
        E5: Debug + Display,
        E6: Debug + Display,
    {
    }

    #[derive(Debug)]
    pub enum EitherError7<E1, E2, E3, E4, E5, E6, E7> {
        E1(E1),
        E2(E2),
        E3(E3),
        E4(E4),
        E5(E5),
        E6(E6),
        E7(E7),
    }

    impl<E1, E2, E3, E4, E5, E6, E7> Display for EitherError7<E2, E1, E3, E4, E5, E6, E7>
    where
        E1: Display,
        E2: Display,
        E3: Display,
        E4: Display,
        E5: Display,
        E6: Display,
        E7: Display,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                Self::E1(e) => write!(f, "Error {}", e),
                Self::E2(e) => write!(f, "Error {}", e),
                Self::E3(e) => write!(f, "Error {}", e),
                Self::E4(e) => write!(f, "Error {}", e),
                Self::E5(e) => write!(f, "Error {}", e),
                Self::E6(e) => write!(f, "Error {}", e),
                Self::E7(e) => write!(f, "Error {}", e),
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
                Self::E1(e) => e.kind(),
                Self::E2(e) => e.kind(),
                Self::E3(e) => e.kind(),
                Self::E4(e) => e.kind(),
                Self::E5(e) => e.kind(),
                Self::E6(e) => e.kind(),
                Self::E7(e) => e.kind(),
            }
        }
    }

    #[cfg(feature = "std")]
    impl<E1, E2, E3, E4, E5, E6, E7> std::error::Error for EitherError7<E1, E2, E3, E4, E5, E6, E7>
    where
        E1: Debug + Display,
        E2: Debug + Display,
        E3: Debug + Display,
        E4: Debug + Display,
        E5: Debug + Display,
        E6: Debug + Display,
        E7: Debug + Display,
    {
    }

    #[derive(Debug)]
    pub enum EitherError8<E1, E2, E3, E4, E5, E6, E7, E8> {
        E1(E1),
        E2(E2),
        E3(E3),
        E4(E4),
        E5(E5),
        E6(E6),
        E7(E7),
        E8(E8),
    }

    impl<E1, E2, E3, E4, E5, E6, E7, E8> Display for EitherError8<E2, E1, E3, E4, E5, E6, E7, E8>
    where
        E1: Display,
        E2: Display,
        E3: Display,
        E4: Display,
        E5: Display,
        E6: Display,
        E7: Display,
        E8: Display,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                Self::E1(e) => write!(f, "Error {}", e),
                Self::E2(e) => write!(f, "Error {}", e),
                Self::E3(e) => write!(f, "Error {}", e),
                Self::E4(e) => write!(f, "Error {}", e),
                Self::E5(e) => write!(f, "Error {}", e),
                Self::E6(e) => write!(f, "Error {}", e),
                Self::E7(e) => write!(f, "Error {}", e),
                Self::E8(e) => write!(f, "Error {}", e),
            }
        }
    }

    impl<E1, E2, E3, E4, E5, E6, E7, E8> Error for EitherError8<E1, E2, E3, E4, E5, E6, E7, E8>
    where
        E1: Error,
        E2: Error,
        E3: Error,
        E4: Error,
        E5: Error,
        E6: Error,
        E7: Error,
        E8: Error,
    {
        fn kind(&self) -> ErrorKind {
            match self {
                Self::E1(e) => e.kind(),
                Self::E2(e) => e.kind(),
                Self::E3(e) => e.kind(),
                Self::E4(e) => e.kind(),
                Self::E5(e) => e.kind(),
                Self::E6(e) => e.kind(),
                Self::E7(e) => e.kind(),
                Self::E8(e) => e.kind(),
            }
        }
    }

    #[cfg(feature = "std")]
    impl<E1, E2, E3, E4, E5, E6, E7, E8> std::error::Error
        for EitherError8<E1, E2, E3, E4, E5, E6, E7, E8>
    where
        E1: Debug + Display,
        E2: Debug + Display,
        E3: Debug + Display,
        E4: Debug + Display,
        E5: Debug + Display,
        E6: Debug + Display,
        E7: Debug + Display,
        E8: Debug + Display,
    {
    }
}
