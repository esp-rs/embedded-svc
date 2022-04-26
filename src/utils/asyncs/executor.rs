use core::future::Future;
use core::marker::PhantomData;
use core::ptr;

extern crate alloc;
use alloc::sync::Arc;

use async_task::{Runnable, Task};

use crossbeam_queue::ArrayQueue;

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
    fn prerun(&self) {}

    fn notify(&self);

    fn postrun(&self) {}
}

impl<F> Notify for F
where
    F: Fn(),
{
    fn notify(&self) {
        (self)()
    }
}

pub struct LocalExecutor<'a, W, N>(Executor<'a, W, N>, *const ());

impl<'a, W, N> LocalExecutor<'a, W, N>
where
    W: Wait + 'a,
    N: Notify + Clone + Send + 'a,
{
    pub fn new(size: usize, wait: W, notify: N) -> Self {
        Self(Executor::new(size, wait, notify), ptr::null())
    }

    pub fn spawn<F, T>(&mut self, fut: F) -> Task<T>
    where
        F: Future<Output = T> + 'a,
        T: 'a,
    {
        unsafe { self.0.spawn(fut) }
    }

    pub fn run<C, T>(&mut self, until: C, tasks: Option<T>)
    where
        C: Fn() -> bool,
    {
        unsafe { self.0.run(until, tasks) }
    }
}

pub struct SendableExecutor<'a, W, N>(Executor<'a, W, N>);

impl<'a, W, N> SendableExecutor<'a, W, N>
where
    W: Wait + 'a,
    N: Notify + Clone + Send + 'a,
{
    pub fn new(size: usize, wait: W, notify: N) -> Self {
        Self(Executor::new(size, wait, notify))
    }

    pub fn spawn<F, T>(&mut self, fut: F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'a,
        T: Send + 'a,
    {
        unsafe { self.0.spawn(fut) }
    }

    pub fn run<C, T>(&mut self, until: C, tasks: Option<T>)
    where
        C: Fn() -> bool,
    {
        unsafe { self.0.run(until, tasks) }
    }
}

pub struct Executor<'a, W, N> {
    queue: Arc<ArrayQueue<Runnable>>,
    wait: W,
    notify: N,
    _lft: PhantomData<&'a ()>,
}

impl<'a, W, N> Executor<'a, W, N>
where
    W: Wait + 'a,
    N: Notify + Clone + Send + 'a,
{
    pub fn new(size: usize, wait: W, notify: N) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(size)),
            wait,
            notify,
            _lft: PhantomData,
        }
    }

    pub unsafe fn spawn<F, T>(&mut self, fut: F) -> Task<T>
    where
        F: Future<Output = T> + 'a,
        T: 'a,
    {
        let schedule = {
            let queue = self.queue.clone();
            let notify = self.notify.clone();

            move |runnable| {
                queue.push(runnable).unwrap();
                notify.notify();
            }
        };

        let (runnable, task) = async_task::spawn_unchecked(fut, schedule);

        runnable.schedule();

        task
    }

    pub unsafe fn run<C, T>(&mut self, until: C, tasks: Option<T>)
    where
        C: Fn() -> bool,
    {
        self.notifier().prerun();

        while !until() {
            if !self.tick() {
                self.wait();
            }
        }

        if let Some(tasks) = tasks {
            drop(tasks);

            while self.tick() {}
        }

        self.notifier().postrun();
    }

    pub unsafe fn tick(&mut self) -> bool {
        if let Some(runnable) = self.queue.pop() {
            runnable.run();

            true
        } else {
            false
        }
    }

    pub unsafe fn wait(&mut self) {
        self.wait.wait();
    }

    pub unsafe fn notifier(&mut self) -> &N {
        &self.notify
    }
}
