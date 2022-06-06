#[cfg(feature = "experimental")]
pub mod asynch {
    use core::fmt::Debug;
    use core::future::Future;
    use core::result::Result;

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
