use core::future::Future;
use core::mem;
use core::result::Result;
use core::stream::Stream;
use core::task::{Poll, Waker};
use core::time::Duration;
use std::marker::PhantomData;

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

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
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

                mem::replace(&mut state.waker, None).map(Waker::wake);
            }

            Result::<_, Self::Error>::Ok(())
        })?;

        state.lock().timer = Some(timer);

        Ok(OnceFuture(state))
    }
}

pub struct EveryState<T> {
    timer: Option<T>,
    due: bool,
    waker: Option<Waker>,
}

pub struct EveryStream<T, MX>(Arc<MX>)
where
    MX: Mutex<Data = EveryState<T>>;

impl<T, MX> Stream for EveryStream<T, MX>
where
    T: crate::timer::Timer,
    MX: Mutex<Data = EveryState<T>>,
{
    type Item = Result<(), T::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut state = self.0.lock();

        if state.due {
            Poll::Ready(Some(Ok(())))
        } else {
            let first_waker = mem::replace(&mut state.waker, Some(cx.waker().clone())).is_none();

            if first_waker {
                if let Some(timer) = &mut state.timer {
                    let result = timer.start();
                    if result.is_err() {
                        return Poll::Ready(Some(result));
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
    MX: Mutex<Data = EveryState<T::Timer>> + Send + Sync + 'static,
{
    type EveryStream = EveryStream<T::Timer, MX>;

    fn every(&mut self, duration: std::time::Duration) -> Result<Self::EveryStream, Self::Error> {
        let state = Arc::new(MX::new(EveryState {
            timer: None,
            due: false,
            waker: None,
        }));

        let timer_state = Arc::downgrade(&state);

        let timer = self.blocking_periodic.every(duration, move || {
            if let Some(state) = timer_state.upgrade() {
                let mut state = state.lock();

                state.due = true;

                mem::replace(&mut state.waker, None).map(Waker::wake);
            }

            Result::<_, Self::Error>::Ok(())
        })?;

        state.lock().timer = Some(timer);

        Ok(EveryStream(state))
    }
}
