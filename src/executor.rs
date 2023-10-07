pub mod asynch {
    //#[cfg(feature = "nightly")]
    pub use unblocker::*;

    use core::fmt::Debug;

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Unblocking<U, T> {
        pub unblocker: U,
        pub api: T,
    }

    impl<U, T> Unblocking<U, T> {
        pub const fn new(unblocker: U, api: T) -> Self {
            Self { unblocker, api }
        }
    }

    mod unblocker {
        use core::future::Future;

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
    }
}
