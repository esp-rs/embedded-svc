use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::result::Result;
use core::task::{Context, Poll, Waker};
use core::time::Duration;

extern crate alloc;
use alloc::sync::Arc;

use crate::mutex::Mutex;

pub struct OnceState<T> {
    timer: Option<T>,
    due: bool,
    waker: Option<Waker>,
}

pub struct OnceFuture<MX, T>(Arc<MX>)
where
    MX: Mutex<Data = OnceState<T>>;

impl<MX, T> Future for OnceFuture<MX, T>
where
    T: crate::timer::Timer,
    MX: Mutex<Data = OnceState<T>>,
{
    type Output = Result<(), T::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.lock();

        if state.due {
            Poll::Ready(Ok(()))
        } else {
            let first_waker = mem::replace(&mut state.waker, Some(cx.waker().clone())).is_none();

            if first_waker {
                if let Some(timer) = &mut state.timer {
                    let result = timer.start();
                    if result.is_err() {
                        return Poll::Ready(result);
                    }
                } else {
                    panic!();
                }
            }

            Poll::Pending
        }
    }
}

pub struct Once<MX, T> {
    blocking_once: T,
    _mutex_type: PhantomData<fn() -> MX>,
}

impl<MX, T> Once<MX, T>
where
    T: crate::timer::Once,
{
    pub fn new(blocking_once: T) -> Self {
        Self {
            blocking_once,
            _mutex_type: PhantomData,
        }
    }
}

impl<MX, T> Clone for Once<MX, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            blocking_once: self.blocking_once.clone(),
            _mutex_type: PhantomData,
        }
    }
}

impl<MX, T> crate::service::Service for Once<MX, T>
where
    T: crate::service::Service,
{
    type Error = T::Error;
}

impl<MX, T> crate::timer::nonblocking::Once for Once<MX, T>
where
    T: crate::timer::Once,
    T::Timer: Send,
    MX: Mutex<Data = OnceState<T::Timer>> + Send + Sync + 'static,
{
    type AfterFuture = OnceFuture<MX, T::Timer>;

    fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture, Self::Error> {
        let state = Arc::new(MX::new(OnceState {
            timer: None,
            due: false,
            waker: None,
        }));

        let timer_state = Arc::downgrade(&state);

        let timer = self.blocking_once.after(duration, move || {
            if let Some(state) = timer_state.upgrade() {
                let mut state = state.lock();

                state.due = true;

                if let Some(a) = mem::replace(&mut state.waker, None) {
                    Waker::wake(a);
                }
            }

            Result::<_, Self::Error>::Ok(())
        })?;

        state.lock().timer = Some(timer);

        Ok(OnceFuture(state))
    }
}

pub struct TimerState<T> {
    timer: Option<T>,
    due: bool,
    waker: Option<Waker>,
}

pub struct Timer<MX, T>(Arc<MX>)
where
    MX: Mutex<Data = TimerState<T>>;

impl<MX, T> crate::service::Service for Timer<MX, T>
where
    T: crate::timer::Timer,
    MX: Mutex<Data = TimerState<T>>,
{
    type Error = T::Error;
}

impl<MX, T> crate::channel::nonblocking::Receiver for Timer<MX, T>
where
    T: crate::timer::Timer,
    MX: Mutex<Data = TimerState<T>>,
{
    type Data = ();

    type RecvFuture<'a>
    where
        Self: 'a,
    = NextFuture<'a, MX, T>;

    fn recv(&mut self) -> Self::RecvFuture<'_> {
        NextFuture(self)
    }
}

pub struct NextFuture<'a, MX, T>(&'a Timer<MX, T>)
where
    MX: Mutex<Data = TimerState<T>>;

impl<'a, MX, T> Drop for NextFuture<'a, MX, T>
where
    MX: Mutex<Data = TimerState<T>>,
{
    fn drop(&mut self) {
        let mut state = self.0 .0.lock();

        state.due = false;
        state.waker = None;
    }
}

impl<'a, MX, T> Future for NextFuture<'a, MX, T>
where
    T: crate::timer::Timer,
    MX: Mutex<Data = TimerState<T>>,
{
    type Output = Result<(), T::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0 .0.lock();

        if state.due {
            Poll::Ready(Ok(()))
        } else {
            let first_waker = mem::replace(&mut state.waker, Some(cx.waker().clone())).is_none();

            if first_waker {
                if let Some(timer) = &mut state.timer {
                    let result = timer.start();
                    if result.is_err() {
                        return Poll::Ready(result);
                    }
                } else {
                    panic!();
                }
            }

            Poll::Pending
        }
    }
}

pub struct Periodic<MX, T> {
    blocking_periodic: T,
    _mutex_type: PhantomData<fn() -> MX>,
}

impl<MX, T> Periodic<MX, T>
where
    T: crate::timer::Periodic,
{
    pub fn new(blocking_periodic: T) -> Self {
        Self {
            blocking_periodic,
            _mutex_type: PhantomData,
        }
    }
}

impl<MX, T> Clone for Periodic<MX, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            blocking_periodic: self.blocking_periodic.clone(),
            _mutex_type: PhantomData,
        }
    }
}

impl<MX, T> crate::service::Service for Periodic<MX, T>
where
    T: crate::service::Service,
{
    type Error = T::Error;
}

impl<MX, T> crate::timer::nonblocking::Periodic for Periodic<MX, T>
where
    T: crate::timer::Periodic,
    T::Timer: Send,
    MX: Mutex<Data = TimerState<T::Timer>> + Send + Sync + 'static,
{
    type Timer = Timer<MX, T::Timer>;

    fn every(&mut self, duration: Duration) -> Result<Self::Timer, Self::Error> {
        let state = Arc::new(MX::new(TimerState {
            timer: None,
            due: false,
            waker: None,
        }));

        let timer_state = Arc::downgrade(&state);

        let timer = self.blocking_periodic.every(duration, move || {
            if let Some(state) = timer_state.upgrade() {
                let mut state = state.lock();

                state.due = true;

                if let Some(a) = mem::replace(&mut state.waker, None) {
                    Waker::wake(a);
                }
            }

            Result::<_, Self::Error>::Ok(())
        })?;

        state.lock().timer = Some(timer);

        Ok(Timer(state))
    }
}
