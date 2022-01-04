use core::ops::{Deref, DerefMut};

/// A simple Mutex trait for no_std environments.
///
/// Unlike [mutex-trait](https://github.com/rust-embedded/wg/blob/master/rfcs/0377-mutex-trait.md)
/// this one does NOT take &mut self in its locking method.
///
/// This makes it compatible with core::sync::Arc, i.e. it can be passed around to threads freely.
///
/// Note that the implementation uses Rust GATs, which requires nightly, but the hope is that GATs will be stabilized soon.
pub trait Mutex {
    /// Data protected by the mutex.
    type Data;

    type Guard<'a>: Deref<Target = Self::Data> + DerefMut<Target = Self::Data>
    where
        Self::Data: 'a,
        Self: 'a;

    fn new(data: Self::Data) -> Self;

    fn lock(&self) -> Self::Guard<'_>;
}

#[cfg(feature = "std")]
impl<T> Mutex for std::sync::Mutex<T> {
    type Data = T;

    type Guard<'a>
    where
        T: 'a,
    = std::sync::MutexGuard<'a, T>;

    #[inline(always)]
    fn new(data: Self::Data) -> Self {
        std::sync::Mutex::new(data)
    }

    #[inline(always)]
    fn lock(&self) -> Self::Guard<'_> {
        std::sync::Mutex::lock(self).unwrap()
    }
}
