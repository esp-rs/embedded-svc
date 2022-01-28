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

pub struct OnceFuture<T, MX>(Arc<MX>)
where
    MX: Mutex<Data = OnceState<T>>;

impl<T, MX> Future for OnceFuture<T, MX>
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

pub struct Once<T, MX> {
    blocking_once: T,
    _mutex_type: PhantomData<fn() -> MX>,
}

impl<T, MX> crate::service::Service for Once<T, MX>
where
    T: crate::service::Service,
{
    type Error = T::Error;
}

impl<T, MX> crate::timer::nonblocking::Once for Once<T, MX>
where
    T: crate::timer::Once,
    T::Timer: Send,
    MX: Mutex<Data = OnceState<T::Timer>> + Send + Sync + 'static,
{
    type AfterFuture = OnceFuture<T::Timer, MX>;

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

pub struct Timer<T, MX>(Arc<MX>)
where
    MX: Mutex<Data = TimerState<T>>;

impl<T, MX> crate::service::Service for Timer<T, MX>
where
    T: crate::timer::Timer,
    MX: Mutex<Data = TimerState<T>>,
{
    type Error = T::Error;
}

impl<T, MX> crate::timer::nonblocking::Timer for Timer<T, MX>
where
    T: crate::timer::Timer,
    MX: Mutex<Data = TimerState<T>>,
{
    type NextFuture<'a>
    where
        Self: 'a,
    = NextFuture<'a, T, MX>;

    fn next(&mut self) -> Self::NextFuture<'_> {
        NextFuture(self)
    }
}

pub struct NextFuture<'a, T, MX>(&'a Timer<T, MX>)
where
    MX: Mutex<Data = TimerState<T>>;

impl<'a, T, MX> Drop for NextFuture<'a, T, MX>
where
    MX: Mutex<Data = TimerState<T>>,
{
    fn drop(&mut self) {
        let mut state = self.0 .0.lock();

        state.due = false;
        state.waker = None;
    }
}

impl<'a, T, MX> Future for NextFuture<'a, T, MX>
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

pub struct Periodic<T, MX> {
    blocking_periodic: T,
    _mutex_type: PhantomData<fn() -> MX>,
}

impl<T, MX> crate::service::Service for Periodic<T, MX>
where
    T: crate::service::Service,
{
    type Error = T::Error;
}

impl<T, MX> crate::timer::nonblocking::Periodic for Periodic<T, MX>
where
    T: crate::timer::Periodic,
    T::Timer: Send,
    MX: Mutex<Data = TimerState<T::Timer>> + Send + Sync + 'static,
{
    type Timer = Timer<T::Timer, MX>;

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
