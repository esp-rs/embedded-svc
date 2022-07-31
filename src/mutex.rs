use core::time::Duration;

/// A raw Mutex trait for no_std environments.
/// An alternative to the Mutex trait that avoids usage of GATs and does not need a MutexFamily (which in turn uses non-lifetime GATs).
pub trait RawMutex {
    fn new() -> Self;

    /// # Safety
    /// - This method should NOT be called while the mutex is being waited on in a condvar
    unsafe fn lock(&self);

    /// # Safety
    /// - This method should NOT be called while the mutex is being waited on in a condvar
    /// - This method should only be called by the entity currently holding the mutex (i.e. the entity which successfully called `lock` earlier)
    unsafe fn unlock(&self);
}

/// A raw Condvar trait for no_std environments.
/// An alternative to the Condvar trait that avoids usage of GATs.
pub trait RawCondvar {
    type RawMutex: RawMutex;

    fn new() -> Self;

    /// # Safety
    /// - This method should be called only when the mutex is already locked, and by the entity which locked the mutex
    unsafe fn wait(&self, mutex: &Self::RawMutex);

    /// # Safety
    /// - This method should be called only when the mutex is already locked, and by the entity which locked the mutex
    unsafe fn wait_timeout(&self, mutex: &Self::RawMutex, duration: Duration) -> bool;

    fn notify_one(&self);

    fn notify_all(&self);
}

pub struct NoopRawMutex;

impl RawMutex for NoopRawMutex {
    fn new() -> Self {
        Self
    }

    unsafe fn lock(&self) {}

    unsafe fn unlock(&self) {}
}

#[cfg(feature = "std")]
pub struct StdRawMutex(
    std::sync::Mutex<()>,
    core::cell::RefCell<Option<std::sync::MutexGuard<'static, ()>>>,
);

#[cfg(feature = "std")]
impl RawMutex for StdRawMutex {
    fn new() -> Self {
        Self(std::sync::Mutex::new(()), core::cell::RefCell::new(None))
    }

    unsafe fn lock(&self) {
        let guard = core::mem::transmute(self.0.lock().unwrap());

        *self.1.borrow_mut() = Some(guard);
    }

    unsafe fn unlock(&self) {
        *self.1.borrow_mut() = None;
    }
}

#[cfg(feature = "std")]
impl Drop for StdRawMutex {
    fn drop(&mut self) {
        unsafe {
            self.unlock();
        }
    }
}

#[cfg(feature = "std")]
pub struct StdRawCondvar(std::sync::Condvar);

#[cfg(feature = "std")]
impl RawCondvar for StdRawCondvar {
    type RawMutex = StdRawMutex;

    fn new() -> Self {
        Self(std::sync::Condvar::new())
    }

    unsafe fn wait(&self, mutex: &Self::RawMutex) {
        let guard = core::mem::replace(&mut *mutex.1.borrow_mut(), None).unwrap();

        let guard = self.0.wait(guard).unwrap();

        *mutex.1.borrow_mut() = Some(guard);
    }

    unsafe fn wait_timeout(&self, mutex: &Self::RawMutex, duration: Duration) -> bool {
        let guard = core::mem::replace(&mut *mutex.1.borrow_mut(), None).unwrap();

        let (guard, wtr) = self.0.wait_timeout(guard, duration).unwrap();

        *mutex.1.borrow_mut() = Some(guard);

        wtr.timed_out()
    }

    fn notify_one(&self) {
        self.0.notify_one();
    }

    fn notify_all(&self) {
        self.0.notify_all();
    }
}

/// A "std-like" Mutex trait for no_std environments.
///
/// Unlike [mutex-trait](https://github.com/rust-embedded/wg/blob/master/rfcs/0377-mutex-trait.md)
/// this one does NOT take &mut self in its locking method.
///
/// This makes it compatible with core::sync::Arc, i.e. it can be passed around to threads freely.
///
/// Note that it uses Rust GATs, which requires nightly, but the hope is that GATs will be stabilized soon.
#[cfg(all(feature = "nightly", feature = "experimental"))]
pub trait Mutex {
    /// Data protected by the mutex.
    type Data;

    type Guard<'a>: core::ops::Deref<Target = Self::Data> + core::ops::DerefMut<Target = Self::Data>
    where
        Self::Data: 'a,
        Self: 'a;

    fn new(data: Self::Data) -> Self;

    fn lock(&self) -> Self::Guard<'_>;
}

/// A HKT trait for specifying mutex types.
/// Note that it uses Rust GATs, which requires nightly, but the hope is that GATs will be stabilized soon.
#[cfg(all(feature = "nightly", feature = "experimental"))]
pub trait MutexFamily {
    type Mutex<T>: Mutex<Data = T>;
}

/// A "std-like" Condvar trait for no_std environments.
/// Note that it uses Rust GATs, which requires nightly, but the hope is that GATs will be stabilized soon.
#[cfg(all(feature = "nightly", feature = "experimental"))]
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

#[cfg(all(feature = "std", feature = "nightly", feature = "experimental"))]
impl<T> Mutex for std::sync::Mutex<T> {
    type Data = T;

    type Guard<'a>
    = std::sync::MutexGuard<'a, T> where T: 'a, Self: 'a;

    #[inline(always)]
    fn new(data: Self::Data) -> Self {
        std::sync::Mutex::new(data)
    }

    #[inline(always)]
    fn lock(&self) -> Self::Guard<'_> {
        std::sync::Mutex::lock(self).unwrap()
    }
}

#[cfg(all(feature = "std", feature = "nightly", feature = "experimental"))]
impl MutexFamily for std::sync::Condvar {
    type Mutex<T> = std::sync::Mutex<T>;
}

#[cfg(all(feature = "std", feature = "nightly", feature = "experimental"))]
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
