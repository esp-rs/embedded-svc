#[cfg(feature = "experimental")]
pub mod asyncs {
    use core::task::{Context, Poll};

    pub trait Signal {
        type Data;

        fn new() -> Self;

        fn reset(&self);

        /// Mark this Signal as completed.
        fn signal(&self, data: Self::Data);

        /// Non-blocking method to retrieve the value of this signal.
        fn try_get(&self) -> Option<Self::Data>;

        /// Non-blocking method to asynchronously wait on this signal.
        fn poll_wait(&self, cx: &mut Context<'_>) -> Poll<Self::Data>;
    }
}
