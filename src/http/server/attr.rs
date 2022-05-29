use core::any::Any;
use core::cell::RefCell;

use crate::errors::Errors;
use crate::storage::DynStorage;

pub struct DynStorageRef<'a, A>(&'a RefCell<A>);

impl<'a, A> DynStorageRef<'a, A> {
    pub fn new(attributes: &'a RefCell<A>) -> Self {
        Self(attributes)
    }
}

impl<'a, A> Errors for DynStorageRef<'a, A>
where
    A: Errors,
{
    type Error = A::Error;
}

impl<'a, A> DynStorage<'a> for DynStorageRef<'a, A>
where
    A: DynStorage<'a>,
{
    fn contains(&self, name: &str) -> Result<bool, Self::Error> {
        self.0.borrow().contains(name)
    }

    fn get(&self, name: &str) -> Result<Option<&'a dyn Any>, Self::Error> {
        self.0.borrow().get(name)
    }

    fn set(&mut self, name: &'a str, value: &'a dyn Any) -> Result<bool, Self::Error> {
        self.0.borrow_mut().set(name, value)
    }

    fn remove(&mut self, name: &str) -> Result<bool, Self::Error> {
        self.0.borrow_mut().remove(name)
    }
}
