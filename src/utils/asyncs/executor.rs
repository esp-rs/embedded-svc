use core::future::Future;
use core::marker::PhantomData;

extern crate alloc;
use alloc::rc::Rc;

use async_task::{Runnable, Task};
use heapless::mpmc::MpMcQueue;

use super::signal::*;
use crate::mutex::SingleThreadedMutex;
use crate::signal::asyncs::Signal;

pub trait Waiter {
    fn wait(&self);
}

impl<F> Waiter for F
where
    F: Fn(),
{
    fn wait(&self) {
        (self)()
    }
}

pub trait Notifier {
    fn notify(&self);
}

impl<F> Notifier for F
where
    F: Fn(),
{
    fn notify(&self) {
        (self)()
    }
}

pub struct LocalExecutor<'a, const S: usize, W, N> {
    queue: MpMcQueue<Runnable, S>,
    waiter: W,
    notifier: N,
    _lft: PhantomData<&'a ()>,
}

impl<'a, const S: usize, W, N> LocalExecutor<'a, S, W, N>
where
    W: Waiter,
    N: Notifier,
{
    pub fn new(waiter: W, notifier: N) -> Self {
        Self {
            queue: MpMcQueue::new(),
            waiter,
            notifier,
            _lft: PhantomData,
        }
    }

    pub fn spawn<T>(&mut self, fut: impl Future<Output = T> + 'a) -> Task<T>
    where
        T: 'a,
    {
        let schedule = |runnable| {
            self.queue.enqueue(runnable).unwrap();
            self.notifier.notify();
        };

        let (runnable, task) = unsafe { async_task::spawn_unchecked(fut, schedule) };

        runnable.schedule();

        task
    }

    pub fn run<T>(&mut self, fut: impl Future<Output = T> + 'a) -> T
    where
        T: 'a,
    {
        let signal = Rc::new(MutexSignal::<SingleThreadedMutex<_>, _>::new());

        let _task = {
            let signal = signal.clone();

            self.spawn(async move { signal.signal(fut.await) })
        };

        loop {
            if let Some(res) = signal.try_get() {
                return res;
            }

            if let Some(runnable) = self.queue.dequeue() {
                runnable.run();
            }

            self.waiter.wait();
        }
    }
}
