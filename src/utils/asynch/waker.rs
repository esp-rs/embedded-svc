//! (Copied from Embassy with small adaptations)

use core::mem;
use core::task::Waker;

use futures::task::AtomicWaker;

use crate::executor::asynch::WakerRegistration;

/// Utility struct to register and wake a waker.
#[derive(Debug)]
pub struct SingleWakerRegistration {
    waker: Option<Waker>,
}

impl SingleWakerRegistration {
    /// Create a new `WakerRegistration`.
    pub const fn new() -> Self {
        Self { waker: None }
    }

    /// Register a waker. Overwrites the previous waker, if any.
    pub fn register(&mut self, w: &Waker) {
        match self.waker {
            // Optimization: If both the old and new Wakers wake the same task, we can simply
            // keep the old waker, skipping the clone. (In most executor implementations,
            // cloning a waker is somewhat expensive, comparable to cloning an Arc).
            Some(ref w2) if (w2.will_wake(w)) => {}
            _ => {
                // clone the new waker and store it
                if let Some(old_waker) = mem::replace(&mut self.waker, Some(w.clone())) {
                    // We had a waker registered for another task. Wake it, so the other task can
                    // reregister itself if it's still interested.
                    //
                    // If two tasks are waiting on the same thing concurrently, this will cause them
                    // to wake each other in a loop fighting over this WakerRegistration. This wastes
                    // CPU but things will still work.
                    //
                    // If the user wants to have two tasks waiting on the same thing they should use
                    // a more appropriate primitive that can store multiple wakers.
                    old_waker.wake()
                }
            }
        }
    }

    /// Wake the registered waker, if any.
    pub fn wake(&mut self) {
        if let Some(w) = self.waker.take() {
            w.wake()
        }
    }

    /// Returns true if a waker is currently registered
    pub fn occupied(&self) -> bool {
        self.waker.is_some()
    }
}

// Utility struct to register and wake multiple wakers.
pub struct MultiWakerRegistration<const N: usize> {
    wakers: [SingleWakerRegistration; N],
}

impl<const N: usize> MultiWakerRegistration<N> {
    /// Create a new empty instance
    pub const fn new() -> Self {
        const WAKER: SingleWakerRegistration = SingleWakerRegistration::new();
        Self { wakers: [WAKER; N] }
    }

    /// Register a waker. If the buffer is full the function returns it in the error
    pub fn register<'a>(&mut self, w: &'a Waker) -> Result<(), &'a Waker> {
        if let Some(waker_slot) = self
            .wakers
            .iter_mut()
            .find(|waker_slot| !waker_slot.occupied())
        {
            waker_slot.register(w);
            Ok(())
        } else {
            Err(w)
        }
    }

    /// Wake all registered wakers. This clears the buffer
    pub fn wake(&mut self) {
        for waker_slot in self.wakers.iter_mut() {
            waker_slot.wake()
        }
    }
}

impl WakerRegistration for AtomicWaker {
    fn new() -> Self {
        AtomicWaker::new()
    }

    fn register(&mut self, waker: &core::task::Waker) {
        AtomicWaker::register(&self, waker)
    }

    fn wake(&mut self) {
        AtomicWaker::wake(self)
    }
}

impl WakerRegistration for SingleWakerRegistration {
    fn new() -> Self {
        SingleWakerRegistration::new()
    }

    fn register(&mut self, waker: &Waker) {
        SingleWakerRegistration::register(self, waker)
    }

    fn wake(&mut self) {
        SingleWakerRegistration::wake(self)
    }
}

impl<const N: usize> WakerRegistration for MultiWakerRegistration<N> {
    fn new() -> Self {
        MultiWakerRegistration::new()
    }

    fn register(&mut self, waker: &Waker) {
        MultiWakerRegistration::register(self, waker).unwrap()
    }

    fn wake(&mut self) {
        MultiWakerRegistration::wake(self)
    }
}
