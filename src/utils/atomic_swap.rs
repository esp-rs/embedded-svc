use core::cell::Cell;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

pub trait AtomicSwap {
    type Data;

    fn new(data: Self::Data) -> Self;

    fn swap(&self, data: Self::Data) -> Self::Data;
}

pub struct AtomicOption(AtomicBool);

impl AtomicSwap for AtomicOption {
    type Data = Option<()>;

    fn new(data: Self::Data) -> Self {
        Self(AtomicBool::new(data.map(|_| true).unwrap_or(false)))
    }

    fn swap(&self, data: Self::Data) -> Self::Data {
        let data = AtomicBool::swap(
            &self.0,
            data.map(|_| true).unwrap_or(false),
            Ordering::SeqCst,
        );

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

impl<T> AtomicSwap for AtomicPtr<T> {
    type Data = *mut T;

    fn new(data: Self::Data) -> Self {
        AtomicPtr::new(data)
    }

    fn swap(&self, data: Self::Data) -> Self::Data {
        AtomicPtr::swap(self, data, Ordering::SeqCst)
    }
}

macro_rules! atomic_swap {
    ($at:ident, $t:ident) => {
        impl AtomicSwap for core::sync::atomic::$at {
            type Data = $t;

            fn new(data: Self::Data) -> Self {
                core::sync::atomic::$at::new(data)
            }

            fn swap(&self, data: Self::Data) -> Self::Data {
                core::sync::atomic::$at::swap(self, data, Ordering::SeqCst)
            }
        }
    };
}

atomic_swap!(AtomicBool, bool);
atomic_swap!(AtomicU8, u8);
atomic_swap!(AtomicU16, u16);
atomic_swap!(AtomicU32, u32);
atomic_swap!(AtomicUsize, usize);
