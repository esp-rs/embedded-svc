use core::future::Future;

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod event_bus;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod mqtt;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod timer;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod ws;

#[cfg(feature = "alloc")]
pub use blocking_unblocker::*;

// Keep it GAT based for now so that it builds with stable Rust
// and therefore `crate::utils::asyncify` can also build with stable Rust
pub trait Unblocker {
    type UnblockFuture<'a, F, T>: Future<Output = T> + Send
    where
        Self: 'a,
        F: Send + 'a,
        T: Send + 'a;

    fn unblock<'a, F, T>(&'a self, f: F) -> Self::UnblockFuture<'a, F, T>
    where
        F: FnOnce() -> T + Send + 'a,
        T: Send + 'a;
}

impl<U> Unblocker for &U
where
    U: Unblocker,
{
    type UnblockFuture<'a, F, T>
    = U::UnblockFuture<'a, F, T> where Self: 'a, F: Send + 'a, T: Send + 'a;

    fn unblock<'a, F, T>(&'a self, f: F) -> Self::UnblockFuture<'a, F, T>
    where
        F: FnOnce() -> T + Send + 'a,
        T: Send + 'a,
    {
        (*self).unblock(f)
    }
}

impl<U> Unblocker for &mut U
where
    U: Unblocker,
{
    type UnblockFuture<'a, F, T>
    = U::UnblockFuture<'a, F, T> where Self: 'a, F: Send + 'a, T: Send + 'a;

    fn unblock<'a, F, T>(&'a self, f: F) -> Self::UnblockFuture<'a, F, T>
    where
        F: FnOnce() -> T + Send + 'a,
        T: Send + 'a,
    {
        (**self).unblock(f)
    }
}

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

#[cfg(feature = "alloc")]
mod blocking_unblocker {
    use core::future::Future;
    use core::marker::PhantomData;
    use core::task::Poll;

    extern crate alloc;

    use alloc::boxed::Box;

    #[derive(Clone)]
    pub struct BlockingUnblocker(());

    impl BlockingUnblocker {
        pub fn unblock<'a, F, T>(&'a self, f: F) -> BlockingFuture<T>
        where
            F: FnOnce() -> T + Send + 'a,
            T: Send + 'a,
        {
            BlockingFuture::new(f)
        }
    }

    impl super::Unblocker for BlockingUnblocker {
        type UnblockFuture<'a, F, T> = BlockingFuture<'a, T> where Self: 'a, F: Send + 'a, T: Send + 'a;

        fn unblock<'a, F, T>(&'a self, f: F) -> Self::UnblockFuture<'a, F, T>
        where
            F: FnOnce() -> T + Send + 'a,
            T: Send + 'a,
        {
            BlockingUnblocker::unblock(self, f)
        }
    }

    pub fn blocking_unblocker() -> BlockingUnblocker {
        BlockingUnblocker(())
    }

    pub struct BlockingFuture<'a, T> {
        // TODO: Need to box or else we get rustc error:
        // "type parameter `F` is part of concrete type but not used in parameter list for the `impl Trait` type alias"
        computation: Option<Box<dyn FnOnce() -> T + Send + 'a>>,
        _result: PhantomData<fn() -> T>,
    }

    impl<'a, T> BlockingFuture<'a, T> {
        fn new<F>(computation: F) -> Self
        where
            F: FnOnce() -> T + Send + 'a,
            T: Send + 'a,
        {
            Self {
                computation: Some(Box::new(computation)),
                _result: PhantomData,
            }
        }
    }

    impl<'a, T> Future for BlockingFuture<'a, T>
    where
        T: Send + 'a,
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
