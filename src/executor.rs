#[cfg(feature = "experimental")]
pub mod asynch {
    use core::fmt::Debug;
    use core::future::Future;
    use core::result::Result;

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

    pub type TrivialAsync<T> = Blocking<(), T>;

    #[derive(Clone)]
    pub struct Blocking<B, T>(pub(crate) B, pub(crate) T);

    impl<B, T> Blocking<B, T> {
        pub const fn new(blocker: B, api: T) -> Self
        where
            B: Blocker,
        {
            Self(blocker, api)
        }

        pub fn blocker(&self) -> &B {
            &self.0
        }

        pub fn api(&self) -> &T {
            &self.1
        }

        pub fn api_mut(&mut self) -> &mut T {
            &mut self.1
        }
    }

    impl<T> Blocking<(), T> {
        pub const fn new_async(api: T) -> Self {
            Self((), api)
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
