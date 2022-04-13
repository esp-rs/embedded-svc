#[cfg(feature = "experimental")]
pub mod asyncs {
    use core::future::Future;

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
}
