use core::fmt;

#[derive(Debug)]
pub enum SpawnError {
    QueueFull,
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Queue Full Error")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SpawnError {}

#[cfg(all(
    feature = "isr-async-executor",
    feature = "heapless",
    feature = "alloc",
    target_has_atomic = "ptr"
))]
pub mod isr {
    use core::future::Future;
    use core::marker::PhantomData;

    extern crate alloc;
    use alloc::sync::Arc;

    use async_task::{Runnable, Task};

    use heapless::mpmc::MpMcQueue;

    use crate::errors::Errors;
    use crate::executor::asynch::{Executor, LocalSpawner, Spawner, WaitableExecutor};

    use super::SpawnError;

    pub trait Wait {
        fn wait(&self);
    }

    impl<F> Wait for F
    where
        F: Fn(),
    {
        fn wait(&self) {
            (self)()
        }
    }

    pub trait Notify {
        fn notify(&self);
    }

    impl<F> Notify for F
    where
        F: Fn(),
    {
        fn notify(&self) {
            (self)()
        }
    }

    pub trait NotifyFactory {
        type Notify: Notify;

        fn notifier(&self) -> Self::Notify;
    }

    pub trait RunContextFactory {
        fn prerun(&self) {}
        fn postrun(&self) {}
    }

    pub struct RunContext(());

    pub struct ISRExecutor<'a, const C: usize, N, W, S = ()> {
        queue: Arc<MpMcQueue<Runnable, C>>,
        notify_factory: N,
        wait: W,
        _sendable: PhantomData<S>,
        _marker: PhantomData<core::cell::UnsafeCell<&'a ()>>,
    }

    impl<'a, const C: usize, N, W, S> ISRExecutor<'a, C, N, W, S> {
        pub unsafe fn new_unchecked(notify_factory: N, wait: W) -> Self {
            Self {
                queue: Arc::new(MpMcQueue::<_, C>::new()),
                notify_factory,
                wait,
                _sendable: PhantomData,
                _marker: PhantomData,
            }
        }
    }

    impl<'a, const C: usize, N, W, S> ISRExecutor<'a, C, N, W, S>
    where
        N: NotifyFactory,
    {
        pub unsafe fn spawn_unchecked<F, T>(&mut self, fut: F) -> Result<Task<T>, SpawnError>
        where
            F: Future<Output = T>,
        {
            let schedule = {
                let queue = self.queue.clone();
                let notify = self.notify_factory.notifier();

                move |runnable| {
                    queue.enqueue(runnable).unwrap();
                    notify.notify();
                }
            };

            let (runnable, task) = async_task::spawn_unchecked(fut, schedule);

            runnable.schedule();

            Ok(task)
        }
    }

    pub type Local = *const ();

    impl<'a, const C: usize, N, W> ISRExecutor<'a, C, N, W, Local> {
        pub fn new(notify_factory: N, wait: W) -> Self {
            unsafe { Self::new_unchecked(notify_factory, wait) }
        }
    }

    pub type Sendable = ();

    impl<'a, const C: usize, N, W> ISRExecutor<'a, C, N, W, Sendable> {
        pub fn new(notify_factory: N, wait: W) -> Self {
            unsafe { Self::new_unchecked(notify_factory, wait) }
        }
    }

    impl<'a, const C: usize, N, W, S> Errors for ISRExecutor<'a, C, N, W, S> {
        type Error = SpawnError;
    }

    impl<'a, const C: usize, N, W, S> Spawner<'a> for ISRExecutor<'a, C, N, W, S>
    where
        N: NotifyFactory,
    {
        type Task<T>
        where
            T: 'a,
        = Task<T>;

        fn spawn<F, T>(&mut self, fut: F) -> Result<Self::Task<T>, Self::Error>
        where
            F: Future<Output = T> + Send + 'a,
            T: 'a,
        {
            unsafe { self.spawn_unchecked(fut) }
        }
    }

    impl<'a, const C: usize, N, W> LocalSpawner<'a> for ISRExecutor<'a, C, N, W, Local>
    where
        N: NotifyFactory,
    {
        fn spawn_local<F, T>(&mut self, fut: F) -> Result<Self::Task<T>, Self::Error>
        where
            F: Future<Output = T> + 'a,
            T: 'a,
        {
            unsafe { self.spawn_unchecked(fut) }
        }
    }

    impl<'a, const C: usize, N, W, S> Executor for ISRExecutor<'a, C, N, W, S>
    where
        N: RunContextFactory,
    {
        type RunContext = RunContext;

        fn with_context<F, T>(&mut self, run: F) -> T
        where
            F: FnOnce(&mut Self, &RunContext) -> T,
        {
            self.notify_factory.prerun();

            let result = run(self, &RunContext(()));

            self.notify_factory.postrun();

            result
        }

        fn tick(&mut self, _context: &RunContext) -> bool {
            if let Some(runnable) = self.queue.dequeue() {
                runnable.run();

                true
            } else {
                false
            }
        }
    }

    impl<'a, const C: usize, N, W, S> WaitableExecutor for ISRExecutor<'a, C, N, W, S>
    where
        N: RunContextFactory,
        W: Wait,
    {
        fn wait(&mut self, _context: &RunContext) {
            self.wait.wait();
        }
    }
}

#[cfg(feature = "heapless")]
pub mod spawn {
    use core::future::Future;

    use crate::executor::asyncs::{LocalSpawner, Spawner};

    use super::SpawnError;

    pub struct TasksSpawner<'a, const C: usize, S, T>
    where
        S: Spawner<'a>,
        T: 'a,
    {
        spawner: S,
        tasks: heapless::Vec<<S as Spawner<'a>>::Task<T>, C>,
    }

    impl<'a, const C: usize, S, T> TasksSpawner<'a, C, S, T>
    where
        S: Spawner<'a>,
        T: 'a,
    {
        pub fn new(spawner: S) -> Self {
            Self {
                spawner,
                tasks: heapless::Vec::<_, C>::new(),
            }
        }

        pub fn release(self) -> (S, heapless::Vec<<S as Spawner<'a>>::Task<T>, C>) {
            (self.spawner, self.tasks)
        }
    }

    impl<'a, const C: usize, S, T> TasksSpawner<'a, C, S, T>
    where
        S: LocalSpawner<'a>,
        T: 'a,
    {
        pub fn spawn_local<F>(mut self, fut: F) -> Result<Self, SpawnError>
        where
            F: Future<Output = T> + 'a,
        {
            self.spawner
                .spawn_local(fut)
                .map_err(|_| SpawnError::QueueFull)
                .and_then(|task| self.tasks.push(task).map_err(|_| SpawnError::QueueFull))?;

            Ok(self)
        }
    }

    impl<'a, const C: usize, S, T> TasksSpawner<'a, C, S, T>
    where
        S: Spawner<'a>,
        T: 'a,
    {
        pub fn spawn<F>(mut self, fut: F) -> Result<Self, SpawnError>
        where
            F: Future<Output = T> + Send + 'a,
        {
            self.spawner
                .spawn(fut)
                .map_err(|_| SpawnError::QueueFull)
                .and_then(|task| self.tasks.push(task).map_err(|_| SpawnError::QueueFull))?;

            Ok(self)
        }
    }
}
