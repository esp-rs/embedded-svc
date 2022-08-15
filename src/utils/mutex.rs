use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::time::Duration;

/// A raw Mutex trait for no_std environments. Prevents the introduction of dependency on STD for the `utils` module and its sub-modules.
/// NOTE: Users are strongly advised to just depend on STD and use the STD Mutex in their code.
pub trait RawMutex {
    #[cfg(feature = "nightly")] // Remove "nightly" condition once 1.64 is out
    const INIT: Self; // A workaround for not having const fns in traits yet.

    fn new() -> Self;

    /// # Safety
    /// - This method should NOT be called while the mutex is being waited on in a condvar
    unsafe fn lock(&self);

    /// # Safety
    /// - This method should NOT be called while the mutex is being waited on in a condvar
    /// - This method should only be called by the entity currently holding the mutex (i.e. the entity which successfully called `lock` earlier)
    unsafe fn unlock(&self);
}

/// A raw Condvar trait for no_std environments. Prevents the introduction of dependency on STD for the `utils` module and its sub-modules.
/// NOTE: Users are strongly advised to just depend on STD and use the STD Condvar in their code.
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

pub struct Mutex<R, T>(R, UnsafeCell<T>);

impl<R, T> Mutex<R, T>
where
    R: RawMutex,
{
    #[cfg(feature = "nightly")]
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        Self::wrap(R::INIT, data)
    }

    #[cfg(not(feature = "nightly"))]
    #[inline(always)]
    pub fn new(data: T) -> Self {
        Self::wrap(R::new(), data)
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
    R: RawMutex + Send + Sync,
    T: Send,
{
}
unsafe impl<R, T> Send for Mutex<R, T>
where
    R: RawMutex + Send + Sync,
    T: Send,
{
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

// unsafe impl<R, T> Sync for MutexGuard<'_, R, T>
// where
//     R: RawMutex + Send + Sync,
//     T: Sync,
// {
// }

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
    pub fn new() -> Self {
        Self::wrap(V::new())
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

unsafe impl<V> Sync for Condvar<V> where V: RawCondvar + Send + Sync {}
unsafe impl<V> Send for Condvar<V> where V: RawCondvar + Send + Sync {}

impl<V> Default for Condvar<V>
where
    V: RawCondvar,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
pub struct StdRawMutex(
    std::sync::Mutex<()>,
    core::cell::RefCell<Option<std::sync::MutexGuard<'static, ()>>>,
);

#[cfg(feature = "std")]
impl RawMutex for StdRawMutex {
    #[cfg(feature = "nightly")] // Remove "nightly" condition once 1.64 is out
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self(std::sync::Mutex::new(()), core::cell::RefCell::new(None));

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

unsafe impl Send for StdRawMutex {}
unsafe impl Sync for StdRawMutex {}

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
