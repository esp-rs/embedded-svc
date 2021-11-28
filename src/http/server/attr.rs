use core::any::Any;
use core::cell::RefCell;

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use super::Attributes;

pub struct RequestScopedAttributes(BTreeMap<String, Rc<dyn Any>>);

impl RequestScopedAttributes {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

impl<'a> Attributes<'a> for RequestScopedAttributes {
    fn get(&self, name: impl AsRef<str>) -> Option<Rc<dyn Any>> {
        self.0.get(name.as_ref()).map(|value| value.clone())
    }

    fn set(&mut self, name: impl AsRef<str>, value: Rc<dyn Any>) -> Option<Rc<dyn Any>> {
        self.0.insert(name.as_ref().to_owned(), value)
    }

    fn remove(&mut self, name: impl AsRef<str>) -> Option<Rc<dyn Any>> {
        self.0.remove(name.as_ref())
    }
}

pub struct RequestScopedAttributesReference<'a>(&'a RefCell<RequestScopedAttributes>);

impl<'a> RequestScopedAttributesReference<'a> {
    pub fn new(attributes: &'a RefCell<RequestScopedAttributes>) -> Self {
        Self(attributes)
    }
}

impl<'a> Attributes<'a> for RequestScopedAttributesReference<'a> {
    fn get(&self, name: impl AsRef<str>) -> Option<Rc<dyn Any>> {
        self.0.borrow().get(name)
    }

    fn set(&mut self, name: impl AsRef<str>, value: Rc<dyn Any>) -> Option<Rc<dyn Any>> {
        self.0.borrow_mut().set(name, value)
    }

    fn remove(&mut self, name: impl AsRef<str>) -> Option<Rc<dyn Any>> {
        self.0.borrow_mut().remove(name)
    }
}
