#[cfg(feature = "alloc")]
pub use blocking_unblocker::*;

pub trait Unblocker {
    async fn unblock<'a, F, T>(&'a self, f: F) -> T
    where
        F: FnOnce() -> T + Send + 'a,
        T: Send + 'a;
}

impl<U> Unblocker for &U
where
    U: Unblocker,
{
    async fn unblock<'a, F, T>(&'a self, f: F) -> T
    where
        F: FnOnce() -> T + Send + 'a,
        T: Send + 'a,
    {
        (*self).unblock(f).await
    }
}

impl<U> Unblocker for &mut U
where
    U: Unblocker,
{
    async fn unblock<'a, F, T>(&'a self, f: F) -> T
    where
        F: FnOnce() -> T + Send + 'a,
        T: Send + 'a,
    {
        (**self).unblock(f).await
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
        async fn unblock<'a, F, T>(&'a self, f: F) -> T
        where
            F: FnOnce() -> T + Send + 'a,
            T: Send + 'a,
        {
            BlockingUnblocker::unblock(self, f).await
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

    // Temporary, until this issue in Rust nightly is fixed: https://github.com/rust-lang/rust/issues/117602
    unsafe impl<'a, T> Send for BlockingFuture<'a, T> where T: Send + 'a {}
}
