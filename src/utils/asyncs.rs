use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub mod channel;
#[cfg(all(
    feature = "isr-async-executor",
    feature = "alloc",
    target_has_atomic = "ptr"
))]
pub mod executor;
pub mod select;
pub mod signal;

/// Yield from the current task once, allowing other tasks to run.
//
// Code copied from embassy-rs. Smol's futures-lite has an identical implementation.
// Unfortunately, no standard implementation in futures-rs, even though a generic
// executor-independent implementation like the one below is obviously possible.
pub fn yield_now() -> impl Future<Output = ()> {
    YieldNowFuture { yielded: false }
}

struct YieldNowFuture {
    yielded: bool,
}

impl Future for YieldNowFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
