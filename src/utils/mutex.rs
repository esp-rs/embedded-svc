use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::time::Duration;

use crate::mutex::{NoopRawMutex, RawCondvar, RawMutex};

pub struct Mutex<R, T>(R, UnsafeCell<T>);

impl<R, T> Mutex<R, T>
where
    R: RawMutex,
{
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        Self::wrap(R::INIT, data)
    }

    #[inline(always)]
    pub const fn wrap(raw_mutex: R, data: T) -> Self {
        Self(raw_mutex, UnsafeCell::new(data))
    }

    #[inline(always)]
    pub fn lock(&self) -> MutexGuard<'_, R, T> {
        MutexGuard::new(self)
    }
}

unsafe impl<R, T> Sync for Mutex<R, T>
where
    R: RawMutex,
    T: Send,
{
}
unsafe impl<R, T> Send for Mutex<R, T>
where
    R: RawMutex,
    T: Send,
{
}

#[cfg(all(feature = "nightly", feature = "experimental"))]
impl<R, T> crate::mutex::Mutex for Mutex<R, T>
where
    R: RawMutex,
{
    type Data = T;

    type Guard<'a>
    = MutexGuard<'a, R, T> where T: 'a, R: 'a;

    #[inline(always)]
    fn new(data: Self::Data) -> Self {
        Mutex::new(data)
    }

    #[inline(always)]
    fn lock(&self) -> Self::Guard<'_> {
        Mutex::lock(self)
    }
}

pub struct MutexGuard<'a, R, T>(&'a Mutex<R, T>)
where
    R: RawMutex;

impl<'a, R, T> MutexGuard<'a, R, T>
where
    R: RawMutex,
{
    #[inline(always)]
    fn new(mutex: &'a Mutex<R, T>) -> Self {
        unsafe {
            mutex.0.lock();
        }

        Self(mutex)
    }
}

unsafe impl<R, T> Sync for MutexGuard<'_, R, T>
where
    R: RawMutex,
    T: Sync,
{
}

impl<'a, R, T> Drop for MutexGuard<'a, R, T>
where
    R: RawMutex,
{
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            self.0 .0.unlock();
        }
    }
}

impl<'a, R, T> Deref for MutexGuard<'a, R, T>
where
    R: RawMutex,
{
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.0 .1.get().as_mut().unwrap() }
    }
}

impl<'a, R, T> DerefMut for MutexGuard<'a, R, T>
where
    R: RawMutex,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0 .1.get().as_mut().unwrap() }
    }
}

pub struct Condvar<V>(V);

impl<V> Condvar<V>
where
    V: RawCondvar,
{
    pub const fn new() -> Self {
        Self::wrap(V::INIT)
    }

    pub const fn wrap(raw_condvar: V) -> Self {
        Self(raw_condvar)
    }

    pub fn wait<'a, T>(
        &self,
        guard: MutexGuard<'a, V::RawMutex, T>,
    ) -> MutexGuard<'a, V::RawMutex, T> {
        unsafe {
            self.0.wait(&guard.0 .0);
        }

        guard
    }

    pub fn wait_timeout<'a, T>(
        &self,
        guard: MutexGuard<'a, V::RawMutex, T>,
        duration: Duration,
    ) -> (MutexGuard<'a, V::RawMutex, T>, bool) {
        let timeout = unsafe { self.0.wait_timeout(&guard.0 .0, duration) };

        (guard, timeout)
    }

    pub fn notify_one(&self) {
        self.0.notify_one();
    }

    pub fn notify_all(&self) {
        self.0.notify_all();
    }
}

unsafe impl<V> Sync for Condvar<V> where V: RawCondvar {}
unsafe impl<V> Send for Condvar<V> where V: RawCondvar {}

impl<V> Default for Condvar<V>
where
    V: RawCondvar,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(feature = "nightly", feature = "experimental"))]
impl<V> crate::mutex::MutexFamily for Condvar<V>
where
    V: RawCondvar,
{
    type Mutex<T> = Mutex<V::RawMutex, T>;
}

#[cfg(all(feature = "nightly", feature = "experimental"))]
impl<V> crate::mutex::Condvar for Condvar<V>
where
    V: RawCondvar,
{
    #[inline(always)]
    fn new() -> Self {
        Condvar::new()
    }

    fn wait<'a, T>(
        &self,
        guard: <<Self as crate::mutex::MutexFamily>::Mutex<T> as crate::mutex::Mutex>::Guard<'a>,
    ) -> <<Self as crate::mutex::MutexFamily>::Mutex<T> as crate::mutex::Mutex>::Guard<'a> {
        Condvar::wait(self, guard)
    }

    fn wait_timeout<'a, T>(
        &self,
        guard: <<Self as crate::mutex::MutexFamily>::Mutex<T> as crate::mutex::Mutex>::Guard<'a>,
        duration: Duration,
    ) -> (
        <<Self as crate::mutex::MutexFamily>::Mutex<T> as crate::mutex::Mutex>::Guard<'a>,
        bool,
    ) {
        Condvar::wait_timeout(self, guard, duration)
    }

    fn notify_one(&self) {
        Condvar::notify_one(self)
    }

    fn notify_all(&self) {
        Condvar::notify_all(self)
    }
}

#[cfg(all(feature = "nightly", feature = "experimental"))]
pub struct NoopMutexFamily;

#[cfg(all(feature = "nightly", feature = "experimental"))]
impl crate::mutex::MutexFamily for NoopMutexFamily {
    type Mutex<T> = NoopMutex<T>;
}

pub type NoopMutex<T> = crate::utils::mutex::Mutex<NoopRawMutex, T>;
