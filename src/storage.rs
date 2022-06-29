use core::any::Any;
use core::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::errors::wrap::EitherError;

pub trait StorageBase {
    type Error: Debug;

    fn contains(&self, name: &str) -> Result<bool, Self::Error>;
    fn remove(&mut self, name: &str) -> Result<bool, Self::Error>;
}

pub trait Storage: StorageBase {
    fn get<T>(&self, name: &str) -> Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned;

    fn set<T>(&mut self, name: &str, value: &T) -> Result<bool, Self::Error>
    where
        T: serde::Serialize;
}

pub trait DynStorage<'a>: StorageBase {
    fn get(&self, name: &str) -> Result<Option<&'a dyn Any>, Self::Error>;

    fn set(&mut self, name: &'a str, value: &'a dyn Any) -> Result<bool, Self::Error>;
}

pub trait RawStorage: StorageBase {
    fn len(&self, name: &str) -> Result<Option<usize>, Self::Error>;

    fn get_raw<'a>(
        &self,
        name: &str,
        buf: &'a mut [u8],
    ) -> Result<Option<(&'a [u8], usize)>, Self::Error>;

    fn put_raw(&mut self, name: &str, buf: &[u8]) -> Result<bool, Self::Error>;
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

pub struct StorageImpl<const N: usize, R, S> {
    raw_storage: R,
    serde: S,
}

impl<const N: usize, R, S> StorageImpl<N, R, S> {
    pub fn new(raw_storage: R, serde: S) -> Self {
        Self { raw_storage, serde }
    }
}

impl<const N: usize, R, S> StorageBase for StorageImpl<N, R, S>
where
    R: RawStorage,
    S: SerDe,
{
    type Error = EitherError<R::Error, S::Error>;

    fn contains(&self, name: &str) -> Result<bool, Self::Error> {
        Ok(self.raw_storage.contains(name).map_err(EitherError::E1)?)
    }

    fn remove(&mut self, name: &str) -> Result<bool, Self::Error> {
        Ok(self.raw_storage.remove(name).map_err(EitherError::E1)?)
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

        if let Some((buf, _)) = self
            .raw_storage
            .get_raw(name, &mut buf)
            .map_err(EitherError::E1)?
        {
            Ok(Some(self.serde.deserialize(buf).map_err(EitherError::E2)?))
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
            .map_err(EitherError::E2)?;

        Ok(self
            .raw_storage
            .put_raw(name, buf)
            .map_err(EitherError::E1)?)
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
