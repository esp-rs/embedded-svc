/// Zero-copy blocking SPSC channel of one element.
/// Useful as a rendezvous point between two tasks: one - sending, and the other - receiving.
/// Both tasks can wait either in a blocking, or in an async fashion.
///
/// Note that - strictly speaking - the channel is MPSC in the sense that multiple tasks can send data.
/// Doing this in an async fashion however will result in high CPU usage, as the sender tasks will fight over
/// the single sending notification primitive, which supports the registration of only one `Waker`.
use super::mutex::{Condvar, Mutex, RawCondvar};
use super::notification::Notification;

extern crate alloc;
use alloc::sync::Arc;

pub struct Receiver<C, T>(Arc<Channel<C, T>>)
where
    C: RawCondvar;

impl<C, T> Receiver<C, T>
where
    C: RawCondvar,
{
    pub fn get(&mut self) -> Option<&mut T> {
        let mut guard = self.0.state.lock();

        loop {
            match &mut *guard {
                StateData::Empty => guard = self.0.blocking_notify.wait(guard),
                StateData::Quit => break None,
                StateData::Data(data) => break unsafe { (data as *mut T).as_mut() },
            }
        }
    }

    pub async fn get_async(&mut self) -> Option<&mut T> {
        loop {
            {
                let mut guard = self.0.state.lock();

                match &mut *guard {
                    StateData::Empty => (),
                    StateData::Quit => return None,
                    StateData::Data(data) => return unsafe { (data as *mut T).as_mut() },
                }
            }

            self.0.notify_full.wait().await;
        }
    }

    pub fn done(&mut self) {
        let mut guard = self.0.state.lock();

        if matches!(&*guard, StateData::Data(_)) {
            *guard = StateData::Empty;
            self.0.blocking_notify.notify_all();
            self.0.notify_empty.notify();
        }
    }
}

impl<C, T> Drop for Receiver<C, T>
where
    C: RawCondvar,
{
    fn drop(&mut self) {
        let mut guard = self.0.state.lock();

        *guard = StateData::Quit;
        self.0.blocking_notify.notify_all();
        self.0.notify_empty.notify();
    }
}

pub struct Channel<C, T>
where
    C: RawCondvar,
{
    state: Mutex<C::RawMutex, StateData<T>>,
    blocking_notify: Condvar<C>,
    notify_empty: Notification,
    notify_full: Notification,
}

impl<C, T> Channel<C, T>
where
    C: RawCondvar,
{
    pub fn new() -> (Arc<Self>, Receiver<C, T>) {
        let this = Arc::new(Self {
            state: Mutex::new(StateData::Empty),
            blocking_notify: Condvar::new(),
            notify_empty: Notification::new(),
            notify_full: Notification::new(),
        });

        (this.clone(), Receiver(this))
    }

    pub fn set(&self, data: T) -> bool {
        self.set_data(StateData::Data(data))
    }

    pub async fn set_async(&self, data: T) -> bool {
        self.set_data_async(StateData::Data(data)).await
    }

    pub fn quit(&self) {
        self.set_data(StateData::Quit);
    }

    pub async fn quit_async(&self) {
        self.set_data(StateData::Quit);
    }

    fn set_data(&self, data: StateData<T>) -> bool {
        let mut guard = self.state.lock();

        loop {
            match &*guard {
                StateData::Empty => {
                    self.set_data_and_notify(&mut guard, data);
                    break;
                }
                StateData::Quit => return false,
                StateData::Data(_) => guard = self.blocking_notify.wait(guard),
            }
        }

        loop {
            match &*guard {
                StateData::Empty | StateData::Quit => break,
                StateData::Data(_) => guard = self.blocking_notify.wait(guard),
            }
        }

        true
    }

    async fn set_data_async(&self, data: StateData<T>) -> bool {
        loop {
            {
                let mut guard = self.state.lock();

                match &*guard {
                    StateData::Data(_) => (),
                    StateData::Quit => return false,
                    StateData::Empty => {
                        self.set_data_and_notify(&mut *guard, data);
                        break;
                    }
                }
            }

            self.notify_empty.wait().await;
        }

        loop {
            {
                let guard = self.state.lock();

                match &*guard {
                    StateData::Data(_) => (),
                    StateData::Quit | StateData::Empty => break,
                }
            }

            self.notify_empty.wait().await;
        }

        true
    }

    fn set_data_and_notify(&self, cell: &mut StateData<T>, data: StateData<T>) {
        *cell = data;
        self.blocking_notify.notify_all();
        self.notify_full.notify();
    }
}

#[derive(Copy, Clone, Debug)]
enum StateData<T> {
    Empty,
    Data(T),
    Quit,
}
