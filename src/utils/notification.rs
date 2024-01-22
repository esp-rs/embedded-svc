/// Super-simple lockless notification async primitive.
///
/// Notifying is not async and does not block, but overrides the previous notification, if any.
/// Waiting to be notified is async.
///
/// Note that while multiple tasks can await on the notification, this will result in high CPU usage,
/// as the tasks will fight over each other as the notification supports the registration of only one `Waker`
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll};

use atomic_waker::AtomicWaker;

pub struct Notification {
    waker: AtomicWaker,
    notified: AtomicBool,
}

impl Notification {
    pub const fn new() -> Self {
        Self {
            waker: AtomicWaker::new(),
            notified: AtomicBool::new(false),
        }
    }

    pub fn reset(&self) {
        self.notified.store(false, Ordering::SeqCst);
        self.waker.take();
    }

    pub fn notify(&self) {
        self.notified.store(true, Ordering::SeqCst);
        self.waker.wake();
    }

    pub async fn wait(&self) {
        core::future::poll_fn(|cx| self.poll_wait(cx)).await
    }

    fn poll_wait(&self, cx: &Context<'_>) -> Poll<()> {
        self.waker.register(cx.waker());

        if self.notified.swap(false, Ordering::SeqCst) {
            self.waker.take();

            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
