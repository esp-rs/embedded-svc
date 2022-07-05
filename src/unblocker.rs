#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    pub struct Blocking<B, T>(pub(crate) B, pub(crate) T);

    impl<B, T> Blocking<B, T> {
        pub const fn new(blocker: B, api: T) -> Self {
            Self(blocker, api)
        }
    }

    pub trait Blocker {
        fn block_on<F>(&self, f: F) -> F::Output
        where
            F: Future;
    }

    impl<B> Blocker for &B
    where
        B: Blocker,
    {
        fn block_on<F>(&self, f: F) -> F::Output
        where
            F: Future,
        {
            (*self).block_on(f)
        }
    }

    pub trait Unblocker {
        type UnblockFuture<T>: Future<Output = T> + Send
        where
            T: Send;

        fn unblock<F, T>(&self, f: F) -> Self::UnblockFuture<T>
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static;
    }

    impl<U> Unblocker for &U
    where
        U: Unblocker,
    {
        type UnblockFuture<T>
        where
            T: Send,
        = U::UnblockFuture<T>;

        fn unblock<F, T>(&self, f: F) -> Self::UnblockFuture<T>
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            (*self).unblock(f)
        }
    }

    #[cfg(feature = "alloc")]
    pub use utils::*;

    #[cfg(feature = "alloc")]
    mod utils {
        use core::future::Future;
        use core::marker::PhantomData;
        use core::mem;
        use core::task::Poll;

        extern crate alloc;

        use alloc::boxed::Box;

        #[derive(Clone)]
        struct BlockingUnblocker;

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
                let computation = mem::replace(&mut self.computation, None);

                if let Some(computation) = computation {
                    Poll::Ready((computation)())
                } else {
                    unreachable!()
                }
            }
        }

        impl super::Unblocker for BlockingUnblocker {
            type UnblockFuture<T>
            where
                T: Send,
            = BlockingFuture<T>;

            fn unblock<F, T>(&self, f: F) -> Self::UnblockFuture<T>
            where
                F: FnOnce() -> T + Send + 'static,
                T: Send + 'static,
            {
                BlockingFuture::new(f)
            }
        }

        pub fn blocking_unblocker() -> impl super::Unblocker + Clone {
            BlockingUnblocker
        }
    }
}
