use core::future::Future;
use core::mem;
use core::pin::Pin;
use core::result::Result;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll};
use core::time::Duration;

use futures::task::AtomicWaker;

extern crate alloc;
use alloc::sync::Arc;

use crate::channel::nonblocking::Receiver;
use crate::errors::Errors;
use crate::timer::nonblocking::{OnceTimer, PeriodicTimer, TimerService};

pub struct AsyncTimer<T> {
    blocking_timer: T,
    ready: Arc<AtomicBool>,
    waker: Arc<AtomicWaker>,
    duration: Option<Duration>,
}

impl<T> Errors for AsyncTimer<T>
where
    T: Errors,
{
    type Error = T::Error;
}

impl<T> OnceTimer for AsyncTimer<T>
where
    T: crate::timer::OnceTimer + 'static,
{
    type AfterFuture<'a> = TimerFuture<'a, T>;

    fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture<'_>, Self::Error> {
        self.blocking_timer.cancel()?;

        self.waker.take();
        self.ready.store(false, Ordering::SeqCst);
        self.duration = None;

        Ok(TimerFuture(self, Some(duration)))
    }
}

impl<T> PeriodicTimer for AsyncTimer<T>
where
    T: crate::timer::OnceTimer + 'static,
{
    type Clock<'a> = &'a mut Self;

    fn every(&mut self, duration: Duration) -> Result<Self::Clock<'_>, Self::Error> {
        self.blocking_timer.cancel()?;

        self.waker.take();
        self.ready.store(false, Ordering::SeqCst);
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
        self.0.blocking_timer.cancel().unwrap();
    }
}

impl<'a, T> Future for TimerFuture<'a, T>
where
    T: crate::timer::OnceTimer + 'static,
{
    type Output = Result<(), T::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(duration) = mem::replace(&mut self.1, None) {
            match self.0.blocking_timer.after(duration) {
                Ok(_) => (),
                Err(error) => return Poll::Ready(Err(error)),
            }
        }

        self.0.waker.register(cx.waker());

        if self.0.ready.load(Ordering::SeqCst) {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

impl<'a, T> Receiver for &'a mut AsyncTimer<T>
where
    T: crate::timer::OnceTimer + 'static,
{
    type Data = ();

    type RecvFuture<'b>
    where
        'a: 'b,
    = TimerFuture<'b, T>;

    fn recv(&mut self) -> Self::RecvFuture<'_> {
        self.waker.take();
        self.ready.store(false, Ordering::SeqCst);

        TimerFuture(self, self.duration)
    }
}

pub struct AsyncTimerService<T>(T);

impl<T> AsyncTimerService<T> {
    pub fn new(blocking_timer_service: T) -> Self {
        Self(blocking_timer_service)
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

impl<U, T> super::AsyncWrapper<U, T> for AsyncTimerService<T> {
    fn new(blocking_timer_service: T) -> Self {
        AsyncTimerService::new(blocking_timer_service)
    }
}

impl<T> Errors for AsyncTimerService<T>
where
    T: Errors,
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
        let ready = Arc::new(AtomicBool::new(false));
        let waker = Arc::new(AtomicWaker::new());

        let callback_ready = Arc::downgrade(&ready);
        let callback_waker = Arc::downgrade(&waker);

        let blocking_timer = self.0.timer(move || {
            if let Some(callback_ready) = callback_ready.upgrade() {
                if let Some(callback_waker) = callback_waker.upgrade() {
                    callback_ready.store(true, Ordering::SeqCst);
                    callback_waker.wake();
                }
            }
        })?;

        Ok(Self::Timer {
            blocking_timer,
            ready,
            waker,
            duration: None,
        })
    }
}
