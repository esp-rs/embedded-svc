#[cfg(feature = "experimental")]
pub mod nonblocking {
    use core::future::Future;

    pub trait Blocker {
        fn block<F>(f: F) -> F::Output
        where
            F: Future;
    }

    pub trait Unblocker {
        type UnblockFuture<T>: Future<Output = T>;

        // TODO: Need to box f or else - when we implement the Unblocker trait by calling `smol::unblock()` we get:
        // "type parameter `F` is part of concrete type but not used in parameter list for the `impl Trait` type alias"
        fn unblock<T>(f: Box<dyn FnOnce() -> T + Send + 'static>) -> Self::UnblockFuture<T>
        where
            T: Send + 'static;
    }
}
