use core::future::Future;
use core::mem;
use core::result::Result;
use core::stream::Stream;
use core::task::{Poll, Waker};
use core::time::Duration;

extern crate alloc;
use alloc::sync::Arc;

use std::sync::Mutex;

pub struct OnceState<T> {
    timer: Option<T>,
    due: bool,
    waker: Option<Waker>,
}

pub struct Once<T>(Arc<Mutex<OnceState<T>>>);

impl<T> Future for Once<T>
where
    T: crate::timer::Timer,
{
    type Output = Result<(), T::Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.lock().unwrap();

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

impl<T> crate::timer::nonblocking::Once for T
where
    T: crate::timer::Once,
    T::Timer: Send,
{
    type AfterFuture = Once<T::Timer>;

    fn after(&mut self, duration: Duration) -> Result<Self::AfterFuture, Self::Error> {
        let state = Arc::new(Mutex::new(OnceState {
            timer: None,
            due: false,
            waker: None,
        }));

        let timer_state = Arc::downgrade(&state);

        let timer = crate::timer::Once::after(self, duration, move || {
            if let Some(state) = timer_state.upgrade() {
                let mut state = state.lock().unwrap();

                state.due = true;

                mem::replace(&mut state.waker, None).map(Waker::wake);
            }

            Result::<_, Self::Error>::Ok(())
        })?;

        state.lock().unwrap().timer = Some(timer);

        Ok(Once(state))
    }
}

pub struct EveryState<T> {
    timer: Option<T>,
    due: bool,
    waker: Option<Waker>,
}

pub struct Every<T>(Arc<Mutex<EveryState<T>>>);

impl<T> Stream for Every<T>
where
    T: crate::timer::Timer,
{
    type Item = Result<(), T::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut state = self.0.lock().unwrap();

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

impl<T> crate::timer::nonblocking::Periodic for T
where
    T: crate::timer::Periodic,
    T::Timer: Send,
{
    type EveryStream = Every<T::Timer>;

    fn every(&mut self, duration: std::time::Duration) -> Result<Self::EveryStream, Self::Error> {
        let state = Arc::new(Mutex::new(EveryState {
            timer: None,
            due: false,
            waker: None,
        }));

        let timer_state = Arc::downgrade(&state);

        let timer = crate::timer::Periodic::every(self, duration, move || {
            if let Some(state) = timer_state.upgrade() {
                let mut state = state.lock().unwrap();

                state.due = true;

                mem::replace(&mut state.waker, None).map(Waker::wake);
            }

            Result::<_, Self::Error>::Ok(())
        })?;

        state.lock().unwrap().timer = Some(timer);

        Ok(Every(state))
    }
}
