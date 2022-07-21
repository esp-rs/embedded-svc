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
impl ::std::error::Error for SpawnError {}

#[cfg(all(
    feature = "embedded-async-executor",
    feature = "alloc",
    target_has_atomic = "ptr"
))]
pub mod embedded {
    use core::future::Future;
    use core::marker::PhantomData;
    use core::task::{Context, Poll};

    extern crate alloc;
    use alloc::rc::Rc;
    use alloc::sync::Arc;

    use async_task::{Runnable, Task};

    use heapless::mpmc::MpMcQueue;

    use crate::executor::asynch::{Executor, LocalSpawner, Spawner, WaitableExecutor};
    use crate::mutex::RawCondvar;
    use crate::unblocker::asynch::Blocker;
    use crate::utils::mutex::{Condvar, Mutex};

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

    pub trait Notify: Send + Sync {
        fn notify(&self);
    }

    impl<F> Notify for F
    where
        F: Fn() + Send + Sync,
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

    #[derive(Clone)]
    pub struct CondvarWait<R>(Rc<Mutex<R::RawMutex, ()>>, Arc<Condvar<R>>)
    where
        R: RawCondvar;

    impl<R> CondvarWait<R>
    where
        R: RawCondvar,
    {
        pub fn new() -> Self {
            Self(Rc::new(Mutex::new(())), Arc::new(Condvar::new()))
        }

        pub fn notify_factory(&self) -> Arc<Condvar<R>> {
            self.1.clone()
        }
    }

    impl<R> Wait for CondvarWait<R>
    where
        R: RawCondvar + Send + Sync,
    {
        fn wait(&self) {
            let guard = self.0.lock();

            self.1.wait(guard);
        }
    }

    impl<R> NotifyFactory for Arc<Condvar<R>>
    where
        R: RawCondvar + Send + Sync,
    {
        type Notify = Self;

        fn notifier(&self) -> Self::Notify {
            self.clone()
        }
    }

    impl<R> RunContextFactory for Arc<Condvar<R>> where R: RawCondvar + Send + Sync {}

    impl<R> Notify for Arc<Condvar<R>>
    where
        R: RawCondvar + Send + Sync,
    {
        fn notify(&self) {
            self.notify_one();
        }
    }

    struct PrivateData;

    pub struct RunContext(PrivateData);

    /// WORK IN PROGRESS
    /// `EmbeddedExecutor` is an implementation of the [Spawner], [LocalSpawner], [Executor] and [Blocker] traits
    /// that is useful specfically for embedded environments.
    ///
    /// The implementation is in fact a thin wrapper around [smol](::smol)'s [async-task](::async-task) crate.
    ///
    /// Highlights:
    /// - `no_std` (but does need `alloc`; for a `no_std` *and* "no_alloc" executor, look at [Embassy](::embassy), which statically pre-allocates all tasks);
    ///            (note also that usage of `alloc` is very limited - only when a new task is being spawn, as well as the executor itself);
    /// - Does not assume an RTOS and can run completely bare-metal (or on top of an RTOS);
    /// - Pluggable [Wait] & [Notify] mechanism which makes it ISR-friendly. In particular:
    ///   - Tasks can be woken up (and thus re-scheduled) from within an ISR;
    ///   - The executor itself can run not just in the main "thread", but from within an ISR as well;
    ///     the latter is important for bare-metal non-RTOS use-cases, as it allows - by running multiple executors -
    ///     one in the main "thread" and the others - on ISR interrupts - to achieve RTOS-like pre-emptive execution by scheduling higher-priority
    ///     tasks in executors that run on (higher-level) ISRs and thus pre-empt the executors
    ///     scheduled on lower-level ISR and the "main thread" one; note that deploying an executor in an ISR requires the allocator to be usable
    ///     from an ISR (i.e. allocation/deallocation routines should be protected by critical sections that disable/enable interrupts);
    ///   - Out of the box implementations for [Wait] & [Notify] based on condvars, compatible with Rust STD
    ///     (for cases where notifying from / running in ISRs is not important).
    pub struct EmbeddedExecutor<'a, const C: usize, N, W, S = ()> {
        queue: Arc<MpMcQueue<Runnable, C>>,
        notify_factory: N,
        wait: W,
        _sendable: PhantomData<S>,
        _marker: PhantomData<core::cell::UnsafeCell<&'a ()>>,
    }

    impl<'a, const C: usize, N, W, S> EmbeddedExecutor<'a, C, N, W, S> {
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

    impl<'a, const C: usize, N, W, S> EmbeddedExecutor<'a, C, N, W, S>
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

    impl<'a, const C: usize, N, W> EmbeddedExecutor<'a, C, N, W, Local> {
        pub fn new(notify_factory: N, wait: W) -> Self {
            unsafe { Self::new_unchecked(notify_factory, wait) }
        }
    }

    pub type Sendable = ();

    impl<'a, const C: usize, N, W> EmbeddedExecutor<'a, C, N, W, Sendable> {
        pub fn new(notify_factory: N, wait: W) -> Self {
            unsafe { Self::new_unchecked(notify_factory, wait) }
        }
    }

    impl<'a, const C: usize, N, W, S> Spawner<'a> for EmbeddedExecutor<'a, C, N, W, S>
    where
        N: NotifyFactory,
    {
        type Error = SpawnError;

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

    impl<'a, const C: usize, N, W> LocalSpawner<'a> for EmbeddedExecutor<'a, C, N, W, Local>
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

    impl<'a, const C: usize, N, W, S> Executor for EmbeddedExecutor<'a, C, N, W, S>
    where
        N: RunContextFactory,
    {
        type RunContext = RunContext;

        fn with_context<F, T>(&mut self, run: F) -> T
        where
            F: FnOnce(&mut Self, &RunContext) -> T,
        {
            self.notify_factory.prerun();

            let result = run(self, &RunContext(PrivateData));

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

    impl<'a, const C: usize, N, W, S> WaitableExecutor for EmbeddedExecutor<'a, C, N, W, S>
    where
        N: RunContextFactory,
        W: Wait,
    {
        fn wait(&mut self, _context: &RunContext) {
            self.wait.wait();
        }
    }

    #[derive(Clone)]
    pub struct EmbeddedBlocker<N, W>(N, W);

    impl<N, W> EmbeddedBlocker<N, W> {
        pub const fn new(notify_factory: N, wait: W) -> Self {
            Self(notify_factory, wait)
        }
    }

    impl<N, W> Blocker for EmbeddedBlocker<N, W>
    where
        N: NotifyFactory,
        N::Notify: 'static,
        W: Wait,
    {
        fn block_on<F>(&self, mut f: F) -> F::Output
        where
            F: Future,
        {
            log::trace!("block_on(): started");

            let mut f = unsafe { core::pin::Pin::new_unchecked(&mut f) };

            let notify = self.0.notifier();

            let waker = waker_fn::waker_fn(move || {
                notify.notify();
            });

            let cx = &mut Context::from_waker(&waker);

            loop {
                if let Poll::Ready(t) = f.as_mut().poll(cx) {
                    log::trace!("block_on(): completed");
                    return t;
                }

                self.1.wait();
            }
        }
    }
}

pub mod spawn {
    use core::future::Future;

    use crate::executor::asynch::{LocalSpawner, Spawner};

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
