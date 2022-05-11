use core::cell::Cell;

pub trait AtomicSwap {
    type Data;

    fn new(data: Self::Data) -> Self;

    fn swap(&self, data: Self::Data) -> Self::Data;
}

#[cfg(target_has_atomic = "8")]
pub struct AtomicOption(core::sync::atomic::AtomicBool);

#[cfg(target_has_atomic = "8")]
impl AtomicSwap for AtomicOption {
    type Data = Option<()>;

    fn new(data: Self::Data) -> Self {
        Self(core::sync::atomic::AtomicBool::new(
            data.map(|_| true).unwrap_or(false),
        ))
    }

    fn swap(&self, data: Self::Data) -> Self::Data {
        let data = core::sync::atomic::AtomicBool::swap(
            &self.0,
            data.map(|_| true).unwrap_or(false),
            core::sync::atomic::Ordering::SeqCst,
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

#[cfg(target_has_atomic = "ptr")]
impl<T> AtomicSwap for core::sync::atomic::AtomicPtr<T> {
    type Data = *mut T;

    fn new(data: Self::Data) -> Self {
        core::sync::atomic::AtomicPtr::new(data)
    }

    fn swap(&self, data: Self::Data) -> Self::Data {
        core::sync::atomic::AtomicPtr::swap(self, data, core::sync::atomic::Ordering::SeqCst)
    }
}

macro_rules! atomic_swap {
    ($at:ident, $t:ident, $s:literal) => {
        #[cfg(target_has_atomic = $s)]
        impl AtomicSwap for core::sync::atomic::$at {
            type Data = $t;

            fn new(data: Self::Data) -> Self {
                core::sync::atomic::$at::new(data)
            }

            fn swap(&self, data: Self::Data) -> Self::Data {
                core::sync::atomic::$at::swap(self, data, core::sync::atomic::Ordering::SeqCst)
            }
        }
    };
}

atomic_swap!(AtomicBool, bool, "8");
atomic_swap!(AtomicU8, u8, "8");
atomic_swap!(AtomicU16, u16, "16");
atomic_swap!(AtomicU32, u32, "32");
atomic_swap!(AtomicU64, u64, "64");
atomic_swap!(AtomicU128, u128, "128");
atomic_swap!(AtomicI8, i8, "8");
atomic_swap!(AtomicI16, i16, "16");
atomic_swap!(AtomicI32, i32, "32");
atomic_swap!(AtomicI64, i64, "64");
atomic_swap!(AtomicI128, i128, "128");
atomic_swap!(AtomicUsize, usize, "ptr");
