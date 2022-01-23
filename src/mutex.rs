use core::ops::{Deref, DerefMut};
use core::time::Duration;

/// A "std-like" Mutex trait for no_std environments.
///
/// Unlike [mutex-trait](https://github.com/rust-embedded/wg/blob/master/rfcs/0377-mutex-trait.md)
/// this one does NOT take &mut self in its locking method.
///
/// This makes it compatible with core::sync::Arc, i.e. it can be passed around to threads freely.
///
/// Note that it uses Rust GATs, which requires nightly, but the hope is that GATs will be stabilized soon.
pub trait Mutex: Send + Sync {
    /// Data protected by the mutex.
    type Data;

    type Guard<'a>: Deref<Target = Self::Data> + DerefMut<Target = Self::Data>
    where
        Self::Data: 'a,
        Self: 'a;

    fn new(data: Self::Data) -> Self;

    fn lock(&self) -> Self::Guard<'_>;
}

/// A "std-like" Condvar trait for no_std environments.
/// Note that it uses Rust GATs, which requires nightly, but the hope is that GATs will be stabilized soon.
pub trait Condvar: Send + Sync {
    type Mutex<T>: Mutex<Data = T>
    where
        T: Send;

    fn new() -> Self;

    fn wait<'a, T>(
        &self,
        guard: <<Self as Condvar>::Mutex<T> as Mutex>::Guard<'a>,
    ) -> <<Self as Condvar>::Mutex<T> as Mutex>::Guard<'a>
    where
        T: Send;
    fn wait_timeout<'a, T>(
        &self,
        guard: <<Self as Condvar>::Mutex<T> as Mutex>::Guard<'a>,
        duration: Duration,
    ) -> (<<Self as Condvar>::Mutex<T> as Mutex>::Guard<'a>, bool)
    where
        T: Send;

    fn notify_all(&self);
}

#[cfg(feature = "std")]
impl<T> Mutex for std::sync::Mutex<T>
where
    T: Send,
{
    type Data = T;

    type Guard<'a>
    where
        T: 'a,
        Self: 'a,
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

#[cfg(feature = "std")]
impl Condvar for std::sync::Condvar {
    type Mutex<T>
    where
        T: Send,
    = std::sync::Mutex<T>;

    #[inline(always)]
    fn new() -> Self {
        std::sync::Condvar::new()
    }

    #[inline(always)]
    fn wait<'a, T>(
        &self,
        guard: <<Self as Condvar>::Mutex<T> as Mutex>::Guard<'a>,
    ) -> <<Self as Condvar>::Mutex<T> as Mutex>::Guard<'a>
    where
        T: Send,
    {
        std::sync::Condvar::wait(self, guard).unwrap()
    }

    #[inline(always)]
    fn wait_timeout<'a, T>(
        &self,
        guard: <<Self as Condvar>::Mutex<T> as Mutex>::Guard<'a>,
        duration: Duration,
    ) -> (<<Self as Condvar>::Mutex<T> as Mutex>::Guard<'a>, bool)
    where
        T: Send,
    {
        let (guard, timeout_result) =
            std::sync::Condvar::wait_timeout(self, guard, duration).unwrap();

        (guard, timeout_result.timed_out())
    }

    fn notify_all(&self) {
        std::sync::Condvar::notify_all(self);
    }
}
