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

    //#[cfg(feature = "nightly")]
    mod unblocker {
        use core::future::Future;

        // Keep it GAT based for now so that it builds with stable Rust
        // and therefore `crate::utils::asyncify` can also build with stable Rust
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
            = U::UnblockFuture<T> where T: Send;

            fn unblock<F, T>(&self, f: F) -> Self::UnblockFuture<T>
            where
                F: FnOnce() -> T + Send + 'static,
                T: Send + 'static,
            {
                (*self).unblock(f)
            }
        }

        // pub trait Unblocker {
        //     async fn unblock<F, T>(&self, f: F) -> T
        //     where
        //         F: FnOnce() -> T + Send + 'static,
        //         T: Send + 'static;
        // }

        // impl<U> Unblocker for &U
        // where
        //     U: Unblocker,
        // {
        //     async fn unblock<F, T>(&self, f: F) -> T
        //     where
        //         F: FnOnce() -> T + Send + 'static,
        //         T: Send + 'static,
        //     {
        //         (*self).unblock(f).await
        //     }
        // }
    }
}
