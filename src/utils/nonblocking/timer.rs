use core::future::Future;
use core::pin::Pin;
use core::result::Result;
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context, Poll};
use core::time::Duration;

use futures::task::AtomicWaker;

extern crate alloc;
use alloc::sync::Arc;

use crate::channel::nonblocking::Receiver;
use crate::service::Service;
use crate::timer::nonblocking::{OnceTimer, PeriodicTimer, Timer, TimerService};

pub struct AsyncTimer<T> {
    blocking_timer: T,
    ticks: Arc<AtomicU32>,
    waker: Arc<AtomicWaker>,
    skip: bool,
}

impl<T> Service for AsyncTimer<T>
where
    T: Service,
{
    type Error = T::Error;
}

impl<T> Timer for AsyncTimer<T>
where
    T: crate::timer::Timer,
{
    fn is_scheduled(&self) -> Result<bool, Self::Error> {
        self.blocking_timer.is_scheduled()
    }

    fn cancel(&mut self) -> Result<bool, Self::Error> {
        self.blocking_timer.cancel()
    }
}

impl<T> OnceTimer for AsyncTimer<T>
where
    T: crate::timer::OnceTimer + 'static,
{
    type AfterFuture<'a> = &'a mut Self;

    fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture<'_>, Self::Error> {
        self.blocking_timer.cancel()?;

        self.ticks.store(0, Ordering::SeqCst);
        self.waker.take();

        self.blocking_timer.after(duration)?;

        Ok(self)
    }
}

impl<T> PeriodicTimer for AsyncTimer<T>
where
    T: crate::timer::PeriodicTimer + 'static,
{
    type Clock<'a> = &'a mut Self;

    fn every(&mut self, duration: Duration) -> Result<Self::Clock<'_>, Self::Error> {
        self.blocking_timer.cancel()?;

        self.ticks.store(0, Ordering::SeqCst);
        self.waker.take();

        self.blocking_timer.every(duration)?;

        Ok(self)
    }
}

impl<'a, T> Future for &'a mut AsyncTimer<T>
where
    T: Service,
{
    type Output = Result<(), T::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.waker.register(cx.waker());

        loop {
            let value = self.ticks.load(Ordering::SeqCst);
            if value == 0 {
                return Poll::Pending;
            }

            let new_value = if self.skip { 0 } else { value - 1 };

            if self
                .ticks
                .compare_exchange(value, new_value, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Poll::Ready(Ok(()));
            }
        }
    }
}

impl<'a, T> Receiver for &'a mut AsyncTimer<T>
where
    T: Service,
{
    type Data = ();

    type RecvFuture<'b>
    where
        Self: 'b,
    = &'b mut Self;

    fn recv(&mut self) -> Self::RecvFuture<'_> {
        self
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

impl<T> super::AsyncWrapper<T> for AsyncTimerService<T> {
    fn new(blocking_timer_service: T) -> Self {
        AsyncTimerService::new(blocking_timer_service)
    }
}

impl<T> Service for AsyncTimerService<T>
where
    T: Service,
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
        let ticks = Arc::new(AtomicU32::new(0));
        let waker = Arc::new(AtomicWaker::new());

        let callback_ticks = Arc::downgrade(&ticks);
        let callback_waker = Arc::downgrade(&waker);

        let blocking_timer = self.0.timer(move || {
            if let Some(callback_ticks) = callback_ticks.upgrade() {
                if let Some(callback_waker) = callback_waker.upgrade() {
                    callback_ticks.fetch_add(1, Ordering::SeqCst);
                    callback_waker.wake();
                }
            }
        })?;

        Ok(Self::Timer {
            blocking_timer,
            ticks,
            waker,
            skip: false,
        })
    }
}
