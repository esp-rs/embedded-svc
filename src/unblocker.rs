#[cfg(feature = "experimental")]
pub mod asyncs {
    use core::future::{ready, Future, Ready};

    pub trait Blocker<'a> {
        fn block_on<F>(&self, f: F) -> F::Output
        where
            F: Future + 'a;
    }

    pub trait Unblocker {
        type UnblockFuture<T>: Future<Output = T>;

        fn unblock<F, T>(&self, f: F) -> Self::UnblockFuture<T>
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static;
    }

    #[derive(Clone)]
    struct BlockingUnblocker;

    impl Unblocker for BlockingUnblocker {
        type UnblockFuture<T> = Ready<T>;

        fn unblock<F, T>(&self, f: F) -> Self::UnblockFuture<T>
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            ready(f())
        }
    }

    pub fn blocking_unblocker() -> impl Unblocker + Clone {
        BlockingUnblocker
    }
}
