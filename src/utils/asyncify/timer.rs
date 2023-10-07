use core::future::Future;
use core::pin::Pin;
use core::result::Result;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll};
use core::time::Duration;

extern crate alloc;
use alloc::sync::Arc;

use atomic_waker::AtomicWaker;

#[cfg(feature = "nightly")]
pub use async_traits_impl::*;

use super::AsyncWrapper;

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

    fn poll_wait(&self, cx: &Context<'_>) -> Poll<usize> {
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

impl<T> AsyncTimer<T>
where
    T: crate::timer::OnceTimer + Send,
{
    pub async fn after(&mut self, duration: Duration) -> Result<(), T::Error> {
        self.timer.cancel()?;

        self.signal.reset();
        self.duration = None;

        TimerFuture(self, Some(duration)).await;

        Ok(())
    }

    pub fn every(&mut self, duration: Duration) -> Result<&'_ mut Self, T::Error> {
        self.timer.cancel()?;

        self.signal.reset();
        self.duration = Some(duration);

        Ok(self)
    }

    pub async fn tick(&mut self) {
        self.signal.reset();

        TimerFuture(self, self.duration).await
    }
}

struct TimerFuture<'a, T>(&'a mut AsyncTimer<T>, Option<Duration>)
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
    T: crate::timer::OnceTimer,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(duration) = self.1.take() {
            self.0.timer.after(duration).unwrap();
        }

        if self.0.signal.poll_wait(cx).is_ready() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub struct AsyncTimerService<T>(T);

impl<T> AsyncTimerService<T> {
    pub const fn new(timer_service: T) -> Self {
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

impl<T> AsyncTimerService<T>
where
    T: crate::timer::TimerService,
    for<'a> T::Timer<'a>: Send,
{
    pub fn timer(&self) -> Result<AsyncTimer<T::Timer<'_>>, T::Error> {
        let signal = Arc::new(TimerSignal::new());

        let timer = {
            let signal = Arc::downgrade(&signal);

            self.0.timer(move || {
                if let Some(signal) = signal.upgrade() {
                    signal.tick();
                }
            })?
        };

        Ok(AsyncTimer {
            timer,
            signal,
            duration: None,
        })
    }
}

impl<T> AsyncWrapper<T> for AsyncTimerService<T> {
    fn new(timer_service: T) -> Self {
        AsyncTimerService::new(timer_service)
    }
}

#[cfg(feature = "nightly")]
mod async_traits_impl {
    use core::result::Result;
    use core::time::Duration;

    extern crate alloc;

    use crate::timer::asynch::{Clock, ErrorType, OnceTimer, PeriodicTimer, TimerService};

    use super::{AsyncTimer, AsyncTimerService};

    impl<T> ErrorType for AsyncTimer<T>
    where
        T: ErrorType,
    {
        type Error = T::Error;
    }

    impl<T> OnceTimer for AsyncTimer<T>
    where
        T: crate::timer::OnceTimer + Send,
    {
        async fn after(&mut self, duration: Duration) -> Result<(), Self::Error> {
            AsyncTimer::after(self, duration).await
        }
    }

    impl<T> PeriodicTimer for AsyncTimer<T>
    where
        T: crate::timer::OnceTimer + Send,
    {
        type Clock<'a> = &'a mut Self where Self: 'a;

        fn every(&mut self, duration: Duration) -> Result<Self::Clock<'_>, Self::Error> {
            AsyncTimer::every(self, duration)
        }
    }

    impl<'a, T> Clock for &'a mut AsyncTimer<T>
    where
        T: crate::timer::OnceTimer + Send,
    {
        async fn tick(&mut self) {
            AsyncTimer::tick(self).await
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
        for<'a> T::Timer<'a>: Send,
    {
        type Timer<'a> = AsyncTimer<T::Timer<'a>> where Self: 'a;

        async fn timer(&self) -> Result<Self::Timer<'_>, Self::Error> {
            AsyncTimerService::timer(self)
        }
    }

    impl<T> embedded_hal_async::delay::DelayUs for AsyncTimer<T>
    where
        T: crate::timer::OnceTimer + Send + 'static,
    {
        async fn delay_us(&mut self, us: u32) {
            AsyncTimer::after(self, Duration::from_micros(us as _))
                .await
                .unwrap();
        }

        async fn delay_ms(&mut self, ms: u32) {
            AsyncTimer::after(self, Duration::from_millis(ms as _))
                .await
                .unwrap();
        }
    }
}
