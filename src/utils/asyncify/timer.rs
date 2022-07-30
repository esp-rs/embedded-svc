use core::future::Future;
use core::mem;
use core::pin::Pin;
use core::result::Result;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll};
use core::time::Duration;

extern crate alloc;
use alloc::sync::Arc;

use futures::task::AtomicWaker;

use crate::channel::asynch::Receiver;
use crate::timer::asynch::{ErrorType, OnceTimer, PeriodicTimer, TimerService};

struct TimerSignal {
    waker: AtomicWaker,
    ticks: AtomicUsize,
}

impl TimerSignal {
    const fn new() -> Self {
        Self {
            waker: AtomicWaker::new(),
            ticks: AtomicUsize::new(0),
        }
    }

    fn reset(&self) {
        self.ticks.store(0, Ordering::SeqCst);
        self.waker.take();
    }

    fn tick(&self) {
        self.ticks.fetch_add(1, Ordering::SeqCst);
        self.waker.wake();
    }

    fn poll_wait(&self, cx: &mut Context<'_>) -> Poll<usize> {
        self.waker.register(cx.waker());

        let data = self.ticks.swap(0, Ordering::SeqCst);

        if data > 0 {
            Poll::Ready(data)
        } else {
            Poll::Pending
        }
    }
}

pub struct AsyncTimer<T> {
    timer: T,
    signal: Arc<TimerSignal>,
    duration: Option<Duration>,
}

impl<T> ErrorType for AsyncTimer<T>
where
    T: ErrorType,
{
    type Error = T::Error;
}

impl<T> OnceTimer for AsyncTimer<T>
where
    T: crate::timer::OnceTimer + Send + 'static,
{
    type AfterFuture<'a> = TimerFuture<'a, T>;

    fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture<'_>, Self::Error> {
        self.timer.cancel()?;

        self.signal.reset();
        self.duration = None;

        Ok(TimerFuture(self, Some(duration)))
    }
}

impl<T> PeriodicTimer for AsyncTimer<T>
where
    T: crate::timer::OnceTimer + Send + 'static,
{
    type Clock<'a> = &'a mut Self;

    fn every(&mut self, duration: Duration) -> Result<Self::Clock<'_>, Self::Error> {
        self.timer.cancel()?;

        self.signal.reset();
        self.duration = Some(duration);

        Ok(self)
    }
}

pub struct TimerFuture<'a, T>(&'a mut AsyncTimer<T>, Option<Duration>)
where
    T: crate::timer::Timer;

impl<'a, T> Drop for TimerFuture<'a, T>
where
    T: crate::timer::Timer,
{
    fn drop(&mut self) {
        self.0.timer.cancel().unwrap();
    }
}

impl<'a, T> Future for TimerFuture<'a, T>
where
    T: crate::timer::OnceTimer + 'static,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(duration) = mem::replace(&mut self.1, None) {
            self.0.timer.after(duration).unwrap();
        }

        if let Poll::Ready(_) = self.0.signal.poll_wait(cx) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl<'a, T> Receiver for &'a mut AsyncTimer<T>
where
    T: crate::timer::OnceTimer + Send + 'static,
{
    type Data = ();

    type RecvFuture<'b>
    where
        'a: 'b,
    = TimerFuture<'b, T>;

    fn recv(&mut self) -> Self::RecvFuture<'_> {
        self.signal.reset();

        TimerFuture(self, self.duration)
    }
}

pub struct AsyncTimerService<T>(T);

impl<T> AsyncTimerService<T> {
    pub fn new(timer_service: T) -> Self {
        Self(timer_service)
    }
}

impl<T> Clone for AsyncTimerService<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> super::AsyncWrapper<T> for AsyncTimerService<T> {
    fn new(timer_service: T) -> Self {
        AsyncTimerService::new(timer_service)
    }
}

impl<T> ErrorType for AsyncTimerService<T>
where
    T: ErrorType,
{
    type Error = T::Error;
}

impl<T> TimerService for AsyncTimerService<T>
where
    T: crate::timer::TimerService,
    T::Timer: Send,
{
    type Timer = AsyncTimer<T::Timer>;

    fn timer(&mut self) -> Result<Self::Timer, Self::Error> {
        let signal = Arc::new(TimerSignal::new());

        let timer = {
            let signal = Arc::downgrade(&signal);

            self.0.timer(move || {
                if let Some(signal) = signal.upgrade() {
                    signal.tick();
                }
            })?
        };

        Ok(Self::Timer {
            timer,
            signal,
            duration: None,
        })
    }
}
