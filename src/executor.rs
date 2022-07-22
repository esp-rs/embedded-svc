#[cfg(feature = "experimental")]
pub mod asynch {
    use core::fmt::Debug;
    use core::future::Future;
    use core::result::Result;
    use core::task::Waker;

    pub trait WakerRegistration {
        fn new() -> Self;

        fn register(&mut self, waker: &Waker);
        fn wake(&mut self);
    }

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

    #[derive(Clone)]
    pub struct Blocking<B, T> {
        pub blocker: B,
        pub api: T,
    }

    impl<B, T> Blocking<B, T> {
        pub const fn new(blocker: B, api: T) -> Self {
            Self { blocker, api }
        }
    }

    #[derive(Clone)]
    pub struct RawBlocking<B, T> {
        pub blocker: *const B,
        pub api: *mut T,
    }

    impl<B, T> RawBlocking<B, T> {
        pub unsafe fn new() -> Self {
            Self {
                blocker: core::ptr::null(),
                api: core::ptr::null_mut(),
            }
        }
    }

    pub struct TrivialAsync<T> {
        pub api: T,
    }

    impl<T> TrivialAsync<T> {
        pub const fn new(api: T) -> Self {
            Self { api }
        }
    }

    pub struct RawTrivialAsync<T> {
        pub api: *mut T,
    }

    impl<T> RawTrivialAsync<T> {
        pub unsafe fn new() -> Self {
            Self {
                api: core::ptr::null_mut(),
            }
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

    pub trait Spawner<'a> {
        type Error: Debug;

        type Task<T>
        where
            T: 'a;

        fn spawn<F, T>(&mut self, fut: F) -> Result<Self::Task<T>, Self::Error>
        where
            F: Future<Output = T> + Send + 'a,
            T: 'a;
    }

    pub trait LocalSpawner<'a>: Spawner<'a> {
        fn spawn_local<F, T>(&mut self, fut: F) -> Result<Self::Task<T>, Self::Error>
        where
            F: Future<Output = T> + 'a,
            T: 'a;
    }

    pub trait Executor {
        type RunContext;

        fn with_context<F, T>(&mut self, run: F) -> T
        where
            F: FnOnce(&mut Self, &Self::RunContext) -> T;

        fn tick_until<C>(&mut self, context: &Self::RunContext, until: &C) -> bool
        where
            C: Fn() -> bool,
        {
            while !until() {
                if !self.tick(context) {
                    return true;
                }
            }

            false
        }

        fn tick(&mut self, context: &Self::RunContext) -> bool;

        fn drop_tasks<T>(&mut self, context: &Self::RunContext, tasks: T) {
            drop(tasks);

            while self.tick(context) {}
        }
    }

    pub trait WaitableExecutor: Executor {
        fn run<C, T>(&mut self, context: &Self::RunContext, until: C, tasks: Option<T>)
        where
            C: Fn() -> bool,
        {
            self.run_until(context, until);

            if let Some(tasks) = tasks {
                self.drop_tasks(context, tasks)
            }
        }

        fn run_until<C>(&mut self, context: &Self::RunContext, until: C)
        where
            C: Fn() -> bool,
        {
            while self.tick_until(context, &until) {
                self.wait(context);
            }
        }

        fn wait(&mut self, context: &Self::RunContext);
    }
}
