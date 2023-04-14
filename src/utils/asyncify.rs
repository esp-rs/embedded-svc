#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod event_bus;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod mqtt;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod timer;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod ws;

#[cfg(all(feature = "nightly", feature = "experimental"))]
pub use async_wrapper::*;

#[cfg(all(feature = "alloc", feature = "nightly", feature = "experimental"))]
pub use blocking_unblocker::*;

#[cfg(all(feature = "nightly", feature = "experimental"))]
mod async_wrapper {
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
}

#[cfg(all(feature = "alloc", feature = "nightly", feature = "experimental"))]
mod blocking_unblocker {
    use core::future::Future;
    use core::marker::PhantomData;
    use core::task::Poll;

    extern crate alloc;

    use alloc::boxed::Box;

    use crate::executor::asynch::Unblocker;

    #[derive(Clone)]
    struct BlockingUnblocker;

    impl Unblocker for BlockingUnblocker {
        type UnblockFuture<T>
        = BlockingFuture<T> where T: Send;

        fn unblock<F, T>(&self, f: F) -> Self::UnblockFuture<T>
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            BlockingFuture::new(f)
        }
    }

    pub fn blocking_unblocker() -> impl Unblocker + Clone {
        BlockingUnblocker
    }

    pub struct BlockingFuture<T> {
        // TODO: Need to box or else we get rustc error:
        // "type parameter `F` is part of concrete type but not used in parameter list for the `impl Trait` type alias"
        computation: Option<Box<dyn FnOnce() -> T + Send + 'static>>,
        _result: PhantomData<fn() -> T>,
    }

    impl<T> BlockingFuture<T> {
        pub fn new<F>(computation: F) -> Self
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            Self {
                computation: Some(Box::new(computation)),
                _result: PhantomData,
            }
        }
    }

    impl<T> Future for BlockingFuture<T>
    where
        T: Send,
    {
        type Output = T;

        fn poll(
            mut self: core::pin::Pin<&mut Self>,
            _cx: &mut core::task::Context<'_>,
        ) -> Poll<Self::Output> {
            let computation = self.computation.take();

            if let Some(computation) = computation {
                Poll::Ready((computation)())
            } else {
                unreachable!()
            }
        }
    }
}
