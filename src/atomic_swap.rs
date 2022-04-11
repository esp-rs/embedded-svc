use core::sync::atomic::{AtomicBool, Ordering};
use std::{sync::atomic::AtomicU8, cell::Cell};

pub trait AtomicSwap {
    type Data;

    fn new(data: Self::Data) -> Self;

    fn swap(&self, data: Self::Data) -> Self::Data;
}

impl AtomicSwap for AtomicBool {
    type Data = bool;

    fn new(data: Self::Data) -> Self {
        AtomicBool::new(data)
    }
    
    fn swap(&self, data: Self::Data) -> Self::Data {
        AtomicBool::swap(self, data, Ordering::SeqCst)
    }
}

impl AtomicSwap for AtomicU8 {
    type Data = u8;

    fn new(data: Self::Data) -> Self {
        AtomicU8::new(data)
    }
    
    fn swap(&self, data: Self::Data) -> Self::Data {
        AtomicU8::swap(self, data, Ordering::SeqCst)
    }
}

pub struct AtomicOption(AtomicBool);

impl AtomicSwap for AtomicOption {
    type Data = Option<()>;

    fn new(data: Self::Data) -> Self {
        Self(AtomicBool::new(data.map(|_| true).unwrap_or(false)))
    }
    
    fn swap(&self, data: Self::Data) -> Self::Data {
        let data = AtomicBool::swap(&self.0, data.map(|_| true).unwrap_or(false), Ordering::SeqCst);

        data.then(|| ())
    }
}

impl<T> AtomicSwap for Cell<T> {
    type Data = T;

    fn new(data: Self::Data) -> Self {
        Cell::new(data)
    }
    
    fn swap(&self, data: Self::Data) -> Self::Data {
        let swap = Cell::new(data);

        Cell::swap(self, &swap);

        swap.into_inner()
    }
}
