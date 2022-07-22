//! Async mutex.
//!
//! This module provides a mutex that can be used to synchronize data between asynchronous tasks.
//! (Copied from Embassy with small adaptations)
use core::cell::UnsafeCell;
use core::fmt::Display;
use core::ops::{Deref, DerefMut};
use core::task::Poll;

use futures::future::poll_fn;

use crate::executor::asynch::WakerRegistration;
use crate::mutex::RawMutex;

/// Error returned by [`Mutex::try_lock`]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TryLockError;

impl Display for TryLockError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "TryLockError")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TryLockError {}

struct State<W> {
    locked: bool,
    waker: W,
}

/// Async mutex.
///
/// The mutex is generic over a blocking [`RawMutex`](crate::mutex::RawMutex).
/// The raw mutex is used to guard access to the internal "is locked" flag. It
/// is held for very short periods only, while locking and unlocking. It is *not* held
/// for the entire time the async Mutex is locked.
pub struct Mutex<M, W, T>
where
    M: RawMutex,
    T: ?Sized,
{
    state: crate::utils::mutex::Mutex<M, State<W>>,
    inner: UnsafeCell<T>,
}

unsafe impl<M: RawMutex + Send, W: Send, T: ?Sized + Send> Send for Mutex<M, W, T> {}
unsafe impl<M: RawMutex + Sync, W: Send, T: ?Sized + Send> Sync for Mutex<M, W, T> {}

/// Async mutex.
impl<M, W, T> Mutex<M, W, T>
where
    M: RawMutex,
    W: WakerRegistration,
{
    /// Create a new mutex with the given value.
    pub fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
            state: crate::utils::mutex::Mutex::new(State {
                locked: false,
                waker: W::new(),
            }),
        }
    }
}

impl<M, W, T> Mutex<M, W, T>
where
    M: RawMutex,
    W: WakerRegistration,
    T: ?Sized,
{
    /// Lock the mutex.
    ///
    /// This will wait for the mutex to be unlocked if it's already locked.
    pub async fn lock(&self) -> MutexGuard<'_, M, W, T> {
        poll_fn(|cx| {
            let mut state = self.state.lock();

            let ready = if state.locked {
                state.waker.register(cx.waker());
                false
            } else {
                state.locked = true;
                true
            };

            if ready {
                Poll::Ready(MutexGuard { mutex: self })
            } else {
                Poll::Pending
            }
        })
        .await
    }

    /// Attempt to immediately lock the mutex.
    ///
    /// If the mutex is already locked, this will return an error instead of waiting.
    pub fn try_lock(&self) -> Result<MutexGuard<'_, M, W, T>, TryLockError> {
        let mut state = self.state.lock();

        if state.locked {
            Err(TryLockError)
        } else {
            state.locked = true;
            Ok(())
        }?;

        Ok(MutexGuard { mutex: self })
    }
}

/// Async mutex guard.
///
/// Owning an instance of this type indicates having
/// successfully locked the mutex, and grants access to the contents.
///
/// Dropping it unlocks the mutex.
pub struct MutexGuard<'a, M, W, T>
where
    M: RawMutex,
    W: WakerRegistration,
    T: ?Sized,
{
    mutex: &'a Mutex<M, W, T>,
}

impl<'a, M, W, T> Drop for MutexGuard<'a, M, W, T>
where
    M: RawMutex,
    W: WakerRegistration,
    T: ?Sized,
{
    fn drop(&mut self) {
        let mut state = self.mutex.state.lock();

        state.locked = false;
        state.waker.wake();
    }
}

impl<'a, M, W, T> Deref for MutexGuard<'a, M, W, T>
where
    M: RawMutex,
    W: WakerRegistration,
    T: ?Sized,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // Safety: the MutexGuard represents exclusive access to the contents
        // of the mutex, so it's OK to get it.
        unsafe { &*(self.mutex.inner.get() as *const T) }
    }
}

impl<'a, M, W, T> DerefMut for MutexGuard<'a, M, W, T>
where
    M: RawMutex,
    W: WakerRegistration,
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: the MutexGuard represents exclusive access to the contents
        // of the mutex, so it's OK to get it.
        unsafe { &mut *(self.mutex.inner.get()) }
    }
}
