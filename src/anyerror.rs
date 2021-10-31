use anyhow::*;

extern crate alloc;
use alloc::format;

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
