use core::any::Any;
use core::fmt::{self, Debug};

use serde::de::DeserializeOwned;
use serde::Serialize;

pub trait StorageBase {
    type Error: Debug;

    fn contains(&self, name: &str) -> Result<bool, Self::Error>;
    fn remove(&mut self, name: &str) -> Result<bool, Self::Error>;
}

impl<S> StorageBase for &mut S
where
    S: StorageBase,
{
    type Error = S::Error;

    fn contains(&self, name: &str) -> Result<bool, Self::Error> {
        (**self).contains(name)
    }

    fn remove(&mut self, name: &str) -> Result<bool, Self::Error> {
        (*self).remove(name)
    }
}

pub trait Storage: StorageBase {
    fn get<T>(&self, name: &str) -> Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned;

    fn set<T>(&mut self, name: &str, value: &T) -> Result<bool, Self::Error>
    where
        T: serde::Serialize;
}

impl<S> Storage for &mut S
where
    S: Storage,
{
    fn get<T>(&self, name: &str) -> Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        (**self).get(name)
    }

    fn set<T>(&mut self, name: &str, value: &T) -> Result<bool, Self::Error>
    where
        T: serde::Serialize,
    {
        (*self).set(name, value)
    }
}

pub trait DynStorage<'a>: StorageBase {
    fn get(&self, name: &str) -> Result<Option<&'a dyn Any>, Self::Error>;

    fn set(&mut self, name: &'a str, value: &'a dyn Any) -> Result<bool, Self::Error>;
}

impl<'a, D> DynStorage<'a> for &'a mut D
where
    D: DynStorage<'a>,
{
    fn get(&self, name: &str) -> Result<Option<&'a dyn Any>, Self::Error> {
        (**self).get(name)
    }

    fn set(&mut self, name: &'a str, value: &'a dyn Any) -> Result<bool, Self::Error> {
        (*self).set(name, value)
    }
}

pub trait RawStorage: StorageBase {
    fn len(&self, name: &str) -> Result<Option<usize>, Self::Error>;

    fn get_raw<'a>(&self, name: &str, buf: &'a mut [u8]) -> Result<Option<&'a [u8]>, Self::Error>;

    fn put_raw(&mut self, name: &str, buf: &[u8]) -> Result<bool, Self::Error>;
}

impl<R> RawStorage for &mut R
where
    R: RawStorage,
{
    fn len(&self, name: &str) -> Result<Option<usize>, Self::Error> {
        (**self).len(name)
    }

    fn get_raw<'a>(&self, name: &str, buf: &'a mut [u8]) -> Result<Option<&'a [u8]>, Self::Error> {
        (**self).get_raw(name, buf)
    }

    fn put_raw(&mut self, name: &str, buf: &[u8]) -> Result<bool, Self::Error> {
        (**self).put_raw(name, buf)
    }
}

pub trait SerDe {
    type Error: Debug;

    fn serialize<'a, T>(&self, slice: &'a mut [u8], value: &T) -> Result<&'a [u8], Self::Error>
    where
        T: Serialize;

    fn deserialize<T>(&self, slice: &[u8]) -> Result<T, Self::Error>
    where
        T: DeserializeOwned;
}

impl<S> SerDe for &S
where
    S: SerDe,
{
    type Error = S::Error;

    fn serialize<'a, T>(&self, slice: &'a mut [u8], value: &T) -> Result<&'a [u8], Self::Error>
    where
        T: Serialize,
    {
        (*self).serialize(slice, value)
    }

    fn deserialize<T>(&self, slice: &[u8]) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        (*self).deserialize(slice)
    }
}

pub struct StorageImpl<const N: usize, R, S> {
    raw_storage: R,
    serde: S,
}

impl<const N: usize, R, S> StorageImpl<N, R, S> {
    pub fn new(raw_storage: R, serde: S) -> Self {
        Self { raw_storage, serde }
    }
}

#[derive(Debug)]
pub enum StorageError<S, R> {
    SerdeError(S),
    RawStorageError(R),
}

impl<S, R> fmt::Display for StorageError<S, R>
where
    S: fmt::Display,
    R: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerdeError(e) => write!(f, "SerDe error: {}", e),
            Self::RawStorageError(e) => write!(f, "Storage error: {}", e),
        }
    }
}

#[cfg(feature = "std")]
impl<S, R> std::error::Error for StorageError<S, R>
where
    S: std::error::Error,
    R: std::error::Error,
{
}

impl<const N: usize, R, S> StorageBase for StorageImpl<N, R, S>
where
    R: RawStorage,
    S: SerDe,
{
    type Error = StorageError<S::Error, R::Error>;

    fn contains(&self, name: &str) -> Result<bool, Self::Error> {
        self.raw_storage
            .contains(name)
            .map_err(StorageError::RawStorageError)
    }

    fn remove(&mut self, name: &str) -> Result<bool, Self::Error> {
        self.raw_storage
            .remove(name)
            .map_err(StorageError::RawStorageError)
    }
}

impl<const N: usize, R, S> Storage for StorageImpl<N, R, S>
where
    R: RawStorage,
    S: SerDe,
{
    fn get<T>(&self, name: &str) -> Result<Option<T>, Self::Error>
    where
        T: DeserializeOwned,
    {
        let mut buf = [0_u8; N];

        if let Some(buf) = self
            .raw_storage
            .get_raw(name, &mut buf)
            .map_err(StorageError::RawStorageError)?
        {
            Ok(Some(
                self.serde
                    .deserialize(buf)
                    .map_err(StorageError::SerdeError)?,
            ))
        } else {
            Ok(None)
        }
    }

    fn set<T>(&mut self, name: &str, value: &T) -> Result<bool, Self::Error>
    where
        T: Serialize,
    {
        let mut buf = [0_u8; N];

        let buf = self
            .serde
            .serialize(&mut buf, value)
            .map_err(StorageError::SerdeError)?;

        self.raw_storage
            .put_raw(name, buf)
            .map_err(StorageError::RawStorageError)
    }
}

struct Entry<'a> {
    name: &'a str,
    value: &'a dyn Any,
}

pub struct DynStorageImpl<'a, const N: usize>([Option<Entry<'a>>; N]);

#[derive(Debug)]
pub struct NoSpaceError;

impl<'a, const N: usize> StorageBase for DynStorageImpl<'a, N> {
    type Error = NoSpaceError;

    fn contains(&self, name: &str) -> Result<bool, Self::Error> {
        Ok(self.get(name)?.is_some())
    }

    fn remove(&mut self, name: &str) -> Result<bool, Self::Error> {
        if let Some(place) = self.0.iter_mut().find(|entry| {
            entry
                .as_ref()
                .map(|entry| entry.name == name)
                .unwrap_or(false)
        }) {
            *place = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<'a, const N: usize> DynStorage<'a> for DynStorageImpl<'a, N> {
    #[allow(clippy::unnecessary_lazy_evaluations)]
    fn get(&self, name: &str) -> Result<Option<&'a dyn Any>, Self::Error> {
        Ok(self.0.iter().find_map(|entry| {
            entry
                .as_ref()
                .and_then(|entry| (entry.name == name).then(|| entry.value))
        }))
    }

    fn set(&mut self, name: &'a str, value: &'a dyn Any) -> Result<bool, Self::Error> {
        if let Some(entry) = self
            .0
            .iter_mut()
            .find(|entry| {
                entry
                    .as_ref()
                    .map(|entry| entry.name == name)
                    .unwrap_or(false)
            })
            .and_then(|entry| entry.as_mut())
        {
            entry.value = value;
            Ok(true)
        } else if let Some(place) = self.0.iter_mut().find(|entry| entry.is_none()) {
            *place = Some(Entry { name, value });
            Ok(false)
        } else {
            Err(NoSpaceError)
        }
    }
}
