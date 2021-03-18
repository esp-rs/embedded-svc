use std::{cell::RefCell, collections::HashMap, io};

use serde::{Serialize, de::DeserializeOwned};

pub use anyhow::Result;

pub trait Storage {
    fn contains(&self, key: impl AsRef<str>) -> Result<bool>;

    fn remove(&mut self, key: impl AsRef<str>) -> Result<bool>;

    fn len(&self, key: impl AsRef<str>) -> Result<Option<usize>> {
        Ok(self.get_raw(key)?.map(|v| v.len()))
    }

    fn get_raw(&self, key: impl AsRef<str>) -> Result<Option<Vec<u8>>>;

    fn put_raw(&mut self, key: impl AsRef<str>, value: impl Into<Vec<u8>>) -> Result<bool>;

    fn read(&self, key: impl AsRef<str>, into: &mut impl io::Write) -> Result<bool> {
        let data = self.get_raw(key)?;

        Ok(if let Some(data) = data {
            into.write(&data)?;
            true
        } else {
            false
        })
    }

    fn write(&mut self, key: impl AsRef<str>, from: &mut impl io::Read) -> Result<bool> {
        let mut data: Vec<u8> = vec![];

        from.read_to_end(&mut data)?;

        self.put_raw(key, data)
    }

    fn get<T: DeserializeOwned>(&self, key: impl AsRef<str>) -> Result<Option<T>> {
        let data = self.get_raw(key)?;

        Ok(if let Some(data) = data {
            Some(bincode::deserialize::<T>( &data)?)
        } else {
            None
        })
    }

    fn put(&mut self, key: impl AsRef<str>, value: &impl Serialize) -> Result<bool> {
        self.put_raw(key, &*bincode::serialize(value)?)
    }
}

pub struct MemoryStorage(HashMap<String, Vec<u8>>);

impl MemoryStorage {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Storage for MemoryStorage {
    fn contains(&self, key: impl AsRef<str>) -> Result<bool> {
        Ok(self.0.contains_key(key.as_ref()))
    }

    fn remove(&mut self, key: impl AsRef<str>) -> Result<bool> {
        Ok(self.0.remove(key.as_ref()).is_some())
    }

    fn get_raw(&self, key: impl AsRef<str>) -> Result<Option<Vec<u8>>> {
        Ok(self.0.get(key.as_ref()).map(Clone::clone))
    }

    fn put_raw(&mut self, key: impl AsRef<str>, value: impl Into<Vec<u8>>) -> Result<bool> {
        Ok(if let Some(r) = self.0.get_mut(key.as_ref()) {
            *r = value.into();
            true
        } else {
            self.0.insert(key.as_ref().to_owned(), value.into()).is_some()
        })
    }
}

pub struct StorageCache<T: Storage>(T, RefCell<MemoryStorage>);

impl<T: Storage> StorageCache<T> {
    pub fn new(storage: T) -> Self {
        Self(storage, RefCell::new(MemoryStorage::new()))
    }

    pub fn flush(&mut self) {
        *&mut self.1 = RefCell::new(MemoryStorage::new());
    }
}

impl<T: Storage> Storage for StorageCache<T> {
    fn contains(&self, key: impl AsRef<str>) -> Result<bool> {
        Ok(self.1.borrow().contains(&key)? || self.0.contains(key)?)
    }

    fn remove(&mut self, key: impl AsRef<str>) -> Result<bool> {
        self.1.borrow_mut().remove(&key)?;
        self.0.remove(key)
    }

    fn get_raw(&self, key: impl AsRef<str>) -> Result<Option<Vec<u8>>> {
        if let Some(data) = self.1.borrow().get_raw(&key)? {
            Ok(Some(data))
        } else {
            if let Some(data) = self.0.get_raw(&key)? {
                self.1.borrow_mut().put_raw(&key, data.clone())?;
                Ok(Some(data))
            } else {
                self.1.borrow_mut().remove(key)?;
                Ok(None)
            }
        }
    }

    fn put_raw(&mut self, key: impl AsRef<str>, value: impl Into<Vec<u8>>) -> Result<bool> {
        let value = value.into();
        self.1.borrow_mut().put_raw(&key, value.clone())?;
        self.0.put_raw(key, value)
    }
}
