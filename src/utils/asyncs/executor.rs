use core::borrow::Borrow;
use core::future::Future;
use core::marker::PhantomData;
use core::ptr;

extern crate alloc;
use alloc::rc::Rc;
use alloc::sync::Arc;

use async_task::{Runnable, Task};

use crossbeam_queue::ArrayQueue;

use super::signal::*;

use crate::mutex::{MutexFamily, SingleThreadedMutex};
use crate::signal::asyncs::Signal;

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

    pub fn run<F, T>(&mut self, fut: F) -> T
    where
        F: Future<Output = T> + 'a,
        T: 'a,
    {
        self.0.notify.prerun();

        let result = unsafe {
            self.0.run::<_, _, _, MutexSignal<_, _>>(
                fut,
                Rc::new(MutexSignal::<SingleThreadedMutex<_>, _>::new()),
            )
        };

        self.0.notify.postrun();

        result
    }
}

pub struct SendableExecutor<'a, W, N, M>(Executor<'a, W, N>, PhantomData<fn() -> M>);

impl<'a, W, N, M> SendableExecutor<'a, W, N, M>
where
    W: Wait + 'a,
    N: Notify + Clone + Send + 'a,
    M: MutexFamily + Send + Sync + 'a,
{
    pub fn new(size: usize, wait: W, notify: N) -> Self {
        Self(Executor::new(size, wait, notify), PhantomData)
    }

    pub fn spawn<F, T>(&mut self, fut: F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'a,
        T: Send + 'a,
    {
        unsafe { self.0.spawn(fut) }
    }

    pub fn run<F, T>(&mut self, fut: F) -> T
    where
        F: Future<Output = T> + Send + 'a,
        T: Send + 'a,
    {
        self.0.notify.prerun();

        let result = unsafe {
            self.0.run::<_, _, _, MutexSignal<_, _>>(
                fut,
                Arc::new(MutexSignal::<M::Mutex<State<T>>, _>::new()),
            )
        };

        self.0.notify.postrun();

        result
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
    pub fn new(size: usize, waiter: W, notifier: N) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(size)),
            wait: waiter,
            notify: notifier,
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
            let notifier = self.notify.clone();

            move |runnable| {
                queue.push(runnable).unwrap();
                notifier.notify();
            }
        };

        let (runnable, task) = async_task::spawn_unchecked(fut, schedule);

        runnable.schedule();

        task
    }

    pub unsafe fn run<F, T, B, S>(&mut self, fut: F, signal_ref: B) -> T
    where
        F: Future<Output = T> + 'a,
        T: 'a,
        B: Borrow<S> + Clone + 'a,
        S: Signal<Data = T>,
    {
        let _task = {
            let signal_ref = signal_ref.clone();

            self.spawn(async move { signal_ref.borrow().signal(fut.await) })
        };

        loop {
            if let Some(res) = signal_ref.borrow().try_get() {
                return res;
            }

            if let Some(runnable) = self.queue.pop() {
                runnable.run();
            } else {
                self.wait.wait();
            }
        }
    }
}
