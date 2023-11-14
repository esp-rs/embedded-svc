use core::any::Any;
use core::fmt::{self, Debug};

#[cfg(feature = "use_serde")]
use serde::{de::DeserializeOwned, Serialize};

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

#[cfg(feature = "use_serde")]
pub trait Storage: StorageBase {
    fn get<T>(&self, name: &str) -> Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned;

    fn set<T>(&mut self, name: &str, value: &T) -> Result<bool, Self::Error>
    where
        T: serde::Serialize;
}

#[cfg(feature = "use_serde")]
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

    fn set_raw(&mut self, name: &str, buf: &[u8]) -> Result<bool, Self::Error>;
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

    fn set_raw(&mut self, name: &str, buf: &[u8]) -> Result<bool, Self::Error> {
        (**self).set_raw(name, buf)
    }
}

#[cfg(feature = "use_serde")]
pub trait SerDe {
    type Error: Debug;

    fn serialize<'a, T>(&self, slice: &'a mut [u8], value: &T) -> Result<&'a [u8], Self::Error>
    where
        T: Serialize;

    fn deserialize<T>(&self, slice: &[u8]) -> Result<T, Self::Error>
    where
        T: DeserializeOwned;
}

#[cfg(feature = "use_serde")]
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

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StorageError<R, S> {
    RawStorageError(R),
    SerdeError(S),
}

impl<R, S> fmt::Display for StorageError<R, S>
where
    R: fmt::Display,
    S: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RawStorageError(e) => write!(f, "Storage error: {e}"),
            Self::SerdeError(e) => write!(f, "SerDe error: {e}"),
        }
    }
}

#[cfg(feature = "std")]
impl<R, S> std::error::Error for StorageError<R, S>
where
    R: std::error::Error,
    S: std::error::Error,
{
}

#[cfg(feature = "use_serde")]
pub struct StorageImpl<const N: usize, R, S> {
    raw_storage: R,
    serde: S,
}

#[cfg(feature = "use_serde")]
impl<const N: usize, R, S> StorageImpl<N, R, S>
where
    R: RawStorage,
    S: SerDe,
{
    pub const fn new(raw_storage: R, serde: S) -> Self {
        Self { raw_storage, serde }
    }

    pub fn raw_storage(&self) -> &R {
        &self.raw_storage
    }

    pub fn raw_storage_mut(&mut self) -> &mut R {
        &mut self.raw_storage
    }

    pub fn contains(&self, name: &str) -> Result<bool, StorageError<R::Error, S::Error>> {
        self.raw_storage
            .contains(name)
            .map_err(StorageError::RawStorageError)
    }

    pub fn remove(&mut self, name: &str) -> Result<bool, StorageError<R::Error, S::Error>> {
        self.raw_storage
            .remove(name)
            .map_err(StorageError::RawStorageError)
    }

    pub fn get<T>(&self, name: &str) -> Result<Option<T>, StorageError<R::Error, S::Error>>
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

    pub fn set<T>(
        &mut self,
        name: &str,
        value: &T,
    ) -> Result<bool, StorageError<R::Error, S::Error>>
    where
        T: Serialize,
    {
        let mut buf = [0_u8; N];

        let buf = self
            .serde
            .serialize(&mut buf, value)
            .map_err(StorageError::SerdeError)?;

        self.raw_storage
            .set_raw(name, buf)
            .map_err(StorageError::RawStorageError)
    }
}

#[cfg(feature = "use_serde")]
impl<const N: usize, R, S> StorageBase for StorageImpl<N, R, S>
where
    R: RawStorage,
    S: SerDe,
{
    type Error = StorageError<R::Error, S::Error>;

    fn contains(&self, name: &str) -> Result<bool, Self::Error> {
        StorageImpl::contains(self, name)
    }

    fn remove(&mut self, name: &str) -> Result<bool, Self::Error> {
        StorageImpl::remove(self, name)
    }
}

#[cfg(feature = "use_serde")]
impl<const N: usize, R, S> Storage for StorageImpl<N, R, S>
where
    R: RawStorage,
    S: SerDe,
{
    fn get<T>(&self, name: &str) -> Result<Option<T>, Self::Error>
    where
        T: DeserializeOwned,
    {
        StorageImpl::get(self, name)
    }

    fn set<T>(&mut self, name: &str, value: &T) -> Result<bool, Self::Error>
    where
        T: Serialize,
    {
        StorageImpl::set(self, name, value)
    }
}

struct Entry<'a> {
    name: &'a str,
    value: &'a dyn Any,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NoSpaceError;

pub struct DynStorageImpl<'a, const N: usize>([Option<Entry<'a>>; N]);

impl<'a, const N: usize> DynStorageImpl<'a, N> {
    pub fn contains(&self, name: &str) -> Result<bool, NoSpaceError> {
        Ok(self.get(name)?.is_some())
    }

    pub fn remove(&mut self, name: &str) -> Result<bool, NoSpaceError> {
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

    #[allow(clippy::unnecessary_lazy_evaluations)]
    pub fn get(&self, name: &str) -> Result<Option<&'a dyn Any>, NoSpaceError> {
        Ok(self.0.iter().find_map(|entry| {
            entry
                .as_ref()
                .and_then(|entry| (entry.name == name).then(|| entry.value))
        }))
    }

    pub fn set(&mut self, name: &'a str, value: &'a dyn Any) -> Result<bool, NoSpaceError> {
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

impl<'a, const N: usize> StorageBase for DynStorageImpl<'a, N> {
    type Error = NoSpaceError;

    fn contains(&self, name: &str) -> Result<bool, Self::Error> {
        DynStorageImpl::contains(self, name)
    }

    fn remove(&mut self, name: &str) -> Result<bool, Self::Error> {
        DynStorageImpl::remove(self, name)
    }
}

impl<'a, const N: usize> DynStorage<'a> for DynStorageImpl<'a, N> {
    #[allow(clippy::unnecessary_lazy_evaluations)]
    fn get(&self, name: &str) -> Result<Option<&'a dyn Any>, Self::Error> {
        DynStorageImpl::get(self, name)
    }

    fn set(&mut self, name: &'a str, value: &'a dyn Any) -> Result<bool, Self::Error> {
        DynStorageImpl::set(self, name, value)
    }
}
