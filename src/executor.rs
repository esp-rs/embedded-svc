pub mod asynch {
    #[cfg(feature = "nightly")]
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

    #[cfg(feature = "nightly")]
    mod unblocker {
        pub trait Unblocker {
            async fn unblock<F, T>(&self, f: F) -> T
            where
                F: FnOnce() -> T + Send + 'static,
                T: Send + 'static;
        }

        impl<U> Unblocker for &U
        where
            U: Unblocker,
        {
            async fn unblock<F, T>(&self, f: F) -> T
            where
                F: FnOnce() -> T + Send + 'static,
                T: Send + 'static,
            {
                (*self).unblock(f).await
            }
        }
    }
}
