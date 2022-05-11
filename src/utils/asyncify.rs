#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod event_bus;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod mqtt;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod timer;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod ws;

pub trait AsyncWrapper<S> {
    fn new(sync: S) -> Self;
}

pub trait Asyncify {
    type AsyncWrapper<S>: AsyncWrapper<S>;

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

pub trait UnblockingAsyncWrapper<U, S> {
    fn new(unblocker: U, sync: S) -> Self;
}

pub trait UnblockingAsyncify {
    type AsyncWrapper<U, S>: UnblockingAsyncWrapper<U, S>;

    fn unblock_into_async<U>(self, unblocker: U) -> Self::AsyncWrapper<U, Self>
    where
        Self: Sized,
    {
        Self::AsyncWrapper::new(unblocker, self)
    }

    fn unblock_as_async<U>(&mut self, unblocker: U) -> Self::AsyncWrapper<U, &mut Self> {
        Self::AsyncWrapper::new(unblocker, self)
    }
}
