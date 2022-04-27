use core::{future::Future, marker::PhantomData};

extern crate alloc;
use alloc::sync::Arc;

use async_task::{Runnable, Task};

use crossbeam_queue::ArrayQueue;

use crate::executor::asyncs::{Executor, LocalSpawner, Spawner, WaitableExecutor};

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

pub struct ISRExecutor<'a, N, W, S = ()> {
    queue: Arc<ArrayQueue<Runnable>>,
    notify_factory: N,
    wait: W,
    _sendable: PhantomData<S>,
    _marker: PhantomData<core::cell::UnsafeCell<&'a ()>>,
}

impl<'a, N, W, S> ISRExecutor<'a, N, W, S> {
    pub unsafe fn new_unchecked(size: usize, notify_factory: N, wait: W) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(size)),
            notify_factory,
            wait,
            _sendable: PhantomData,
            _marker: PhantomData,
        }
    }
}

impl<'a, N, W, S> ISRExecutor<'a, N, W, S>
where
    N: NotifyFactory,
{
    pub unsafe fn spawn_unchecked<F, T>(&mut self, fut: F) -> Task<T>
    where
        F: Future<Output = T>,
    {
        let schedule = {
            let queue = self.queue.clone();
            let notify = self.notify_factory.notifier();

            move |runnable| {
                queue.push(runnable).unwrap();
                notify.notify();
            }
        };

        let (runnable, task) = async_task::spawn_unchecked(fut, schedule);

        runnable.schedule();

        task
    }
}

pub type Local = *const ();

impl<'a, N, W> ISRExecutor<'a, N, W, Local> {
    pub fn new(size: usize, notify_factory: N, wait: W) -> Self {
        unsafe { Self::new_unchecked(size, notify_factory, wait) }
    }
}

pub type Sendable = ();

impl<'a, N, W> ISRExecutor<'a, N, W, Sendable> {
    pub fn new(size: usize, notify_factory: N, wait: W) -> Self {
        unsafe { Self::new_unchecked(size, notify_factory, wait) }
    }
}

impl<'a, N, W, S> Spawner<'a> for ISRExecutor<'a, N, W, S>
where
    N: NotifyFactory,
{
    type Task<T>
    where
        T: 'a,
    = Task<T>;

    fn spawn<F, T>(&mut self, fut: F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'a,
        T: 'a,
    {
        unsafe { self.spawn_unchecked(fut) }
    }
}

impl<'a, N, W> LocalSpawner<'a> for ISRExecutor<'a, N, W, Local>
where
    N: NotifyFactory,
{
    fn spawn_local<F, T>(&mut self, fut: F) -> Task<T>
    where
        F: Future<Output = T> + 'a,
        T: 'a,
    {
        unsafe { self.spawn_unchecked(fut) }
    }
}

impl<'a, N, W, S> Executor for ISRExecutor<'a, N, W, S>
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
        if let Some(runnable) = self.queue.pop() {
            runnable.run();

            true
        } else {
            false
        }
    }
}

impl<'a, N, W, S> WaitableExecutor for ISRExecutor<'a, N, W, S>
where
    N: RunContextFactory,
    W: Wait,
{
    fn wait(&mut self, _context: &RunContext) {
        self.wait.wait();
    }
}
