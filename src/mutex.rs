use core::cell::{RefCell, RefMut};
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

/// A HKT trait for specifying mutex types.
/// Note that it uses Rust GATs, which requires nightly, but the hope is that GATs will be stabilized soon.
pub trait MutexFamily {
    type Mutex<T>: Mutex<Data = T>;
}

/// A "std-like" Condvar trait for no_std environments.
/// Note that it uses Rust GATs, which requires nightly, but the hope is that GATs will be stabilized soon.
pub trait Condvar: MutexFamily {
    fn new() -> Self;

    fn wait<'a, T>(
        &self,
        guard: <<Self as MutexFamily>::Mutex<T> as Mutex>::Guard<'a>,
    ) -> <<Self as MutexFamily>::Mutex<T> as Mutex>::Guard<'a>;

    fn wait_timeout<'a, T>(
        &self,
        guard: <<Self as MutexFamily>::Mutex<T> as Mutex>::Guard<'a>,
        duration: Duration,
    ) -> (<<Self as MutexFamily>::Mutex<T> as Mutex>::Guard<'a>, bool);

    fn notify_one(&self);

    fn notify_all(&self);
}

pub struct SingleThreadedMutexFamily;

impl MutexFamily for SingleThreadedMutexFamily {
    type Mutex<T> = SingleThreadedMutex<T>;
}

pub struct SingleThreadedMutex<T>(RefCell<T>);

impl<T> SingleThreadedMutex<T> {
    pub fn new(data: T) -> Self {
        Self(RefCell::new(data))
    }

    #[inline(always)]
    pub fn lock(&self) -> SingleThreadedMutexGuard<'_, T> {
        SingleThreadedMutexGuard(self.0.borrow_mut())
    }
}

impl<T> Mutex for SingleThreadedMutex<T> {
    type Data = T;

    type Guard<'a>
    where
        T: 'a,
        Self: 'a,
    = SingleThreadedMutexGuard<'a, T>;

    #[inline(always)]
    fn new(data: Self::Data) -> Self {
        SingleThreadedMutex::new(data)
    }

    #[inline(always)]
    fn lock(&self) -> Self::Guard<'_> {
        SingleThreadedMutex::lock(self)
    }
}

pub struct SingleThreadedMutexGuard<'a, T>(RefMut<'a, T>);

impl<'a, T> Deref for SingleThreadedMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, T> DerefMut for SingleThreadedMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

#[cfg(feature = "std")]
impl<T> Mutex for std::sync::Mutex<T> {
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
impl MutexFamily for std::sync::Condvar {
    type Mutex<T> = std::sync::Mutex<T>;
}

#[cfg(feature = "std")]
impl Condvar for std::sync::Condvar {
    #[inline(always)]
    fn new() -> Self {
        std::sync::Condvar::new()
    }

    #[inline(always)]
    fn wait<'a, T>(
        &self,
        guard: <<Self as MutexFamily>::Mutex<T> as Mutex>::Guard<'a>,
    ) -> <<Self as MutexFamily>::Mutex<T> as Mutex>::Guard<'a> {
        std::sync::Condvar::wait(self, guard).unwrap()
    }

    #[inline(always)]
    fn wait_timeout<'a, T>(
        &self,
        guard: <<Self as MutexFamily>::Mutex<T> as Mutex>::Guard<'a>,
        duration: Duration,
    ) -> (<<Self as MutexFamily>::Mutex<T> as Mutex>::Guard<'a>, bool) {
        let (guard, timeout_result) =
            std::sync::Condvar::wait_timeout(self, guard, duration).unwrap();

        (guard, timeout_result.timed_out())
    }

    fn notify_one(&self) {
        std::sync::Condvar::notify_one(self);
    }

    fn notify_all(&self) {
        std::sync::Condvar::notify_all(self);
    }
}
