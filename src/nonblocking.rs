use core::future::Future;

pub trait Unblocker {
    type UnblockFuture<O>: Future<Output = O>;

    fn unblock<O>(&self, f: impl FnOnce() -> O) -> Self::UnblockFuture<O>
    where
        O: Send + 'static;
}
