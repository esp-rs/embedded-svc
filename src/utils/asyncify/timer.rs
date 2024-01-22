use core::result::Result;
use core::time::Duration;

extern crate alloc;
use alloc::sync::Arc;

use crate::timer::asynch::{Clock, ErrorType, OnceTimer, PeriodicTimer, TimerService};
use crate::utils::notification::Notification;

use super::AsyncWrapper;

pub struct AsyncTimer<T> {
    timer: T,
    notification: Arc<Notification>,
}

impl<T> AsyncTimer<T>
where
    T: crate::timer::OnceTimer + Send,
{
    pub async fn after(&mut self, duration: Duration) -> Result<(), T::Error> {
        self.timer.cancel()?;

        self.notification.reset();
        self.timer.after(duration)?;

        self.notification.wait().await;

        Ok(())
    }
}

impl<T> AsyncTimer<T>
where
    T: crate::timer::PeriodicTimer + Send,
{
    pub fn every(&mut self, duration: Duration) -> Result<&'_ mut Self, T::Error> {
        self.timer.cancel()?;

        self.notification.reset();
        self.timer.every(duration)?;

        Ok(self)
    }

    pub async fn tick(&mut self) {
        self.notification.wait().await;
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
        let notification = Arc::new(Notification::new());

        let timer = {
            let notification = Arc::downgrade(&notification);

            self.0.timer(move || {
                if let Some(notification) = notification.upgrade() {
                    notification.notify();
                }
            })?
        };

        Ok(AsyncTimer {
            timer,
            notification,
        })
    }
}

impl<T> AsyncWrapper<T> for AsyncTimerService<T> {
    fn new(timer_service: T) -> Self {
        AsyncTimerService::new(timer_service)
    }
}

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
    T: crate::timer::PeriodicTimer + Send,
{
    type Clock<'a> = &'a mut Self where Self: 'a;

    fn every(&mut self, duration: Duration) -> Result<Self::Clock<'_>, Self::Error> {
        AsyncTimer::every(self, duration)
    }
}

impl<'a, T> Clock for &'a mut AsyncTimer<T>
where
    T: crate::timer::PeriodicTimer + Send,
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

#[cfg(feature = "embedded-hal-async")]
impl<T> embedded_hal_async::delay::DelayNs for AsyncTimer<T>
where
    T: crate::timer::OnceTimer + Send,
{
    async fn delay_ns(&mut self, ns: u32) {
        AsyncTimer::after(self, Duration::from_micros(ns as _))
            .await
            .unwrap();
    }

    async fn delay_ms(&mut self, ms: u32) {
        AsyncTimer::after(self, Duration::from_millis(ms as _))
            .await
            .unwrap();
    }
}
