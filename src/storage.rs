use core::any::Any;
use core::fmt::Debug;

pub trait StorageBase {
    type Error: Debug;

    fn contains(&self, name: &str) -> Result<bool, Self::Error>;
    fn remove(&mut self, name: &str) -> Result<bool, Self::Error>;
}

pub trait Storage: StorageBase {
    fn get<'a, T>(&'a self, name: &str) -> Result<Option<T>, Self::Error>
    where
        T: serde::Deserialize<'a>;

    fn set<T>(&mut self, name: &str, value: &T) -> Result<bool, Self::Error>
    where
        T: serde::Serialize;
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

pub trait DynStorage<'a>: StorageBase {
    fn get(&self, name: &str) -> Result<Option<&'a dyn Any>, Self::Error>;

    fn set(&mut self, name: &'a str, value: &'a dyn Any) -> Result<bool, Self::Error>;
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
            .map(|entry| entry.as_mut())
            .flatten()
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
