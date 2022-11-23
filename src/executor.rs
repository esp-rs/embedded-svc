#[cfg(feature = "experimental")]
pub mod asynch {
    #[cfg(feature = "nightly")]
    pub use unblocker::*;

    use core::fmt::Debug;
    use core::future::Future;

    pub trait Blocker {
        fn block_on<F>(&self, future: F) -> F::Output
        where
            F: Future;
    }

    impl<B> Blocker for &B
    where
        B: Blocker,
    {
        fn block_on<F>(&self, future: F) -> F::Output
        where
            F: Future,
        {
            (*self).block_on(future)
        }
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Blocking<B, T> {
        pub blocker: B,
        pub api: T,
    }

    impl<B, T> Blocking<B, T> {
        pub const fn new(blocker: B, api: T) -> Self {
            Self { blocker, api }
        }
    }

    #[derive(Clone, Debug)]
    pub struct RawBlocking<B, T> {
        pub blocker: *const B,
        pub api: *mut T,
    }

    impl<B, T> RawBlocking<B, T> {
        pub const fn new() -> Self {
            Self {
                blocker: core::ptr::null(),
                api: core::ptr::null_mut(),
            }
        }
    }

    impl<B, T> Default for RawBlocking<B, T> {
        fn default() -> Self {
            Self::new()
        }
    }

    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct TrivialUnblocking<T> {
        pub api: T,
    }

    impl<T> TrivialUnblocking<T> {
        pub const fn new(api: T) -> Self {
            Self { api }
        }
    }

    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct RawTrivialUnblocking<T> {
        pub api: *mut T,
    }

    impl<T> RawTrivialUnblocking<T> {
        pub const fn new() -> Self {
            Self {
                api: core::ptr::null_mut(),
            }
        }
    }

    impl<T> Default for RawTrivialUnblocking<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(feature = "nightly")]
    mod unblocker {
        use core::future::Future;

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
    }

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

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct RawUnblocking<U, T> {
        pub unblocker: *const U,
        pub api: *mut T,
    }

    impl<U, T> RawUnblocking<U, T> {
        pub fn new() -> Self {
            Self {
                unblocker: core::ptr::null(),
                api: core::ptr::null_mut(),
            }
        }
    }

    impl<B, T> Default for RawUnblocking<B, T> {
        fn default() -> Self {
            Self::new()
        }
    }
}
