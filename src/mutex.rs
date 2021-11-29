/// A simple Mutex trait for no_std environments
/// Unlike [mutex-trait](https://github.com/rust-embedded/wg/blob/master/rfcs/0377-mutex-trait.md)
/// this one does NOT take &mut self in its locking method.
///
/// This makes it compatible with core::sync::Arc, i.e. it can be passed around to threads freely.
pub trait Mutex {
    /// Data protected by the mutex.
    type Data;

    fn new(data: Self::Data) -> Self
    where
        Self: Sized;

    /// Creates a critical section and grants temporary access to the protected data.
    fn with_lock<R>(&self, f: impl FnOnce(&mut Self::Data) -> R) -> R;
}

#[cfg(feature = "std")]
impl<T> Mutex for std::sync::Mutex<T> {
    type Data = T;

    fn new(data: Self::Data) -> Self
    where
        Self: Sized,
    {
        std::sync::Mutex::new(data)
    }

    #[inline(always)]
    fn with_lock<R>(&self, f: impl FnOnce(&mut Self::Data) -> R) -> R {
        let mut guard = self.lock().unwrap();

        f(&mut guard)
    }
}
