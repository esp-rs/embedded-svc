use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::result::Result;
use core::task::{Context, Poll};
use core::time::Duration;

extern crate alloc;
use alloc::sync::Arc;

use crate::channel::asyncs::Receiver;
use crate::errors::Errors;
use crate::signal::asyncs::Signal;
use crate::timer::asyncs::{OnceTimer, PeriodicTimer, TimerService};

pub struct AsyncTimer<T, S> {
    timer: T,
    signal: Arc<S>,
    duration: Option<Duration>,
}

impl<T, S> Errors for AsyncTimer<T, S>
where
    T: Errors,
{
    type Error = T::Error;
}

impl<T, S> OnceTimer for AsyncTimer<T, S>
where
    T: crate::timer::OnceTimer + Send + 'static,
    S: Signal<Data = ()> + Send + Sync + 'static,
{
    type AfterFuture<'a> = TimerFuture<'a, T, S>;

    fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture<'_>, Self::Error> {
        self.timer.cancel()?;

        self.signal.reset();
        self.duration = None;

        Ok(TimerFuture(self, Some(duration)))
    }
}

impl<T, S> PeriodicTimer for AsyncTimer<T, S>
where
    T: crate::timer::OnceTimer + Send + 'static,
    S: Signal<Data = ()> + Send + Sync + 'static,
{
    type Clock<'a> = &'a mut Self;

    fn every(&mut self, duration: Duration) -> Result<Self::Clock<'_>, Self::Error> {
        self.timer.cancel()?;

        self.signal.reset();
        self.duration = Some(duration);

        Ok(self)
    }
}

pub struct TimerFuture<'a, T, S>(&'a mut AsyncTimer<T, S>, Option<Duration>)
where
    T: crate::timer::Timer;

impl<'a, T, S> Drop for TimerFuture<'a, T, S>
where
    T: crate::timer::Timer,
{
    fn drop(&mut self) {
        self.0.timer.cancel().unwrap();
    }
}

impl<'a, T, S> Future for TimerFuture<'a, T, S>
where
    T: crate::timer::OnceTimer + 'static,
    S: Signal<Data = ()>,
{
    type Output = Result<(), T::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(duration) = mem::replace(&mut self.1, None) {
            match self.0.timer.after(duration) {
                Ok(_) => (),
                Err(error) => return Poll::Ready(Err(error)),
            }
        }

        self.0.signal.poll_wait(cx).map(|r| Ok(r))
    }
}

impl<'a, T, S> Receiver for &'a mut AsyncTimer<T, S>
where
    T: crate::timer::OnceTimer + Send + 'static,
    S: Signal<Data = ()> + Send + Sync,
{
    type Data = ();

    type RecvFuture<'b>
    where
        'a: 'b,
    = TimerFuture<'b, T, S>;

    fn recv(&mut self) -> Self::RecvFuture<'_> {
        self.signal.reset();

        TimerFuture(self, self.duration)
    }
}

pub struct AsyncTimerService<T, S>(T, PhantomData<fn() -> S>);

impl<T, S> AsyncTimerService<T, S> {
    pub fn new(timer_service: T) -> Self {
        Self(timer_service, PhantomData)
    }
}

impl<T, S> Clone for AsyncTimerService<T, S>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T, S> super::AsyncWrapper<T> for AsyncTimerService<T, S> {
    fn new(timer_service: T) -> Self {
        AsyncTimerService::new(timer_service)
    }
}

impl<T, S> Errors for AsyncTimerService<T, S>
where
    T: Errors,
{
    type Error = T::Error;
}

impl<T, S> TimerService for AsyncTimerService<T, S>
where
    T: crate::timer::TimerService,
    T::Timer: Send,
    S: Signal<Data = ()> + Send + Sync + 'static,
{
    type Timer = AsyncTimer<T::Timer, S>;

    fn timer(&mut self) -> Result<Self::Timer, Self::Error> {
        let signal = Arc::new(S::new());

        let timer = {
            let signal = Arc::downgrade(&signal);

            self.0.timer(move || {
                if let Some(signal) = signal.upgrade() {
                    signal.signal(());
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
