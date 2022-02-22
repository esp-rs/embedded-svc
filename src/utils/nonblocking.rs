pub mod channel;
#[cfg(feature = "alloc")]
pub mod event_bus;
#[cfg(feature = "alloc")]
pub mod mqtt;
#[cfg(feature = "alloc")]
pub mod signal;
#[cfg(feature = "alloc")]
pub mod timer;

pub trait AsyncWrapper<U, S> {
    fn new(sync: S) -> Self;
}

pub trait Asyncify {
    type AsyncWrapper<S>: AsyncWrapper<(), S>;

    fn into_async(self) -> Self::AsyncWrapper<Self>
    where
        Self: Sized,
    {
        Self::AsyncWrapper::new(self)
    }

    fn as_async(&mut self) -> Self::AsyncWrapper<&mut Self> {
        Self::AsyncWrapper::new(self)
    }
}

pub trait UnblockingAsyncify {
    type AsyncWrapper<U, S>: AsyncWrapper<U, S>;

    fn into_async_with_unblocker<U>(self) -> Self::AsyncWrapper<U, Self>
    where
        Self: Sized,
    {
        Self::AsyncWrapper::new(self)
    }

    fn as_async_with_unblocker<U>(&mut self) -> Self::AsyncWrapper<U, &mut Self> {
        Self::AsyncWrapper::new(self)
    }
}
