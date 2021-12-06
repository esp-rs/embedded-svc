use core::fmt;

use anyhow::{anyhow, Result};

extern crate alloc;

pub struct AnyError<E>(E);

impl<E: core::fmt::Debug> AnyError<E> {
    pub fn into(error: E) -> anyhow::Error {
        anyhow!("Error: {:?}", error)
    }

    pub fn wrap<R, C>(closure: C) -> Result<R>
    where
        C: FnOnce() -> Result<R, E>,
    {
        closure().map_err(Self::into)
    }
}

#[derive(Debug)]
pub struct AsStdError<E: fmt::Debug>(E);

impl<E> fmt::Display for AsStdError<E>
where
    E: fmt::Debug + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<E> From<E> for AsStdError<E>
where
    E: fmt::Debug,
{
    fn from(err: E) -> Self {
        Self(err)
    }
}

#[cfg(feature = "std")]
impl<E> std::error::Error for AsStdError<E>
where
    E: fmt::Display + fmt::Debug,
    // TODO
    // where
    //     E: std::error::Error + 'static,
{
    // TODO
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //    Some(self.0)
    // }
}
