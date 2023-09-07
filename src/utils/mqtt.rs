#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod client {
    use core::fmt::Debug;
    use core::mem;

    use alloc::sync::Arc;

    use crate::utils::mutex::{Condvar, Mutex, RawCondvar};

    use crate::mqtt::client::{ErrorType, Event};

    pub struct ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
    {
        pub state: Mutex<CV::RawMutex, Option<S>>,
        pub state_changed: Condvar<CV>,
    }

    impl<CV, S> ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
    {
        pub fn new(state: S) -> Self {
            Self {
                state: Mutex::new(Some(state)),
                state_changed: Condvar::new(),
            }
        }
    }

    impl<CV, S> ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
        S: Default,
    {
        pub fn new_default() -> Self {
            Self::new(Default::default())
        }
    }

    impl<CV, S> ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
    {
        pub fn close(&self) {
            let mut state = self.state.lock();

            *state = None;
            self.state_changed.notify_all();
        }
    }

    impl<CV, S> Default for ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
        S: Default,
    {
        fn default() -> Self {
            Self::new(Default::default())
        }
    }

    pub struct ConnState<M, E>(Option<Result<Event<M>, E>>);

    impl<M, E> Default for ConnState<M, E> {
        fn default() -> Self {
            Self(Default::default())
        }
    }

    pub struct Postbox<CV, M, E>(Arc<ConnStateGuard<CV, ConnState<M, E>>>)
    where
        CV: RawCondvar;

    impl<CV, M, E> Postbox<CV, M, E>
    where
        CV: RawCondvar,
    {
        pub fn new(connection_state: Arc<ConnStateGuard<CV, ConnState<M, E>>>) -> Self {
            Self(connection_state)
        }

        pub fn post(&mut self, event: Result<Event<M>, E>) {
            let mut state = self.0.state.lock();

            loop {
                if let Some(data) = &mut *state {
                    if data.0.is_some() {
                        state = self.0.state_changed.wait(state);
                    } else {
                        break;
                    }
                } else {
                    return;
                }
            }

            *state = Some(ConnState(Some(event)));
            self.0.state_changed.notify_all();
        }
    }

    pub struct Connection<CV, M, E>(Arc<ConnStateGuard<CV, ConnState<M, E>>>)
    where
        CV: RawCondvar;

    impl<CV, M, E> Connection<CV, M, E>
    where
        CV: RawCondvar,
        E: Debug,
    {
        pub fn new(connection_state: Arc<ConnStateGuard<CV, ConnState<M, E>>>) -> Self {
            Self(connection_state)
        }

        #[allow(clippy::should_implement_trait)]
        pub fn next(&mut self) -> Option<Result<Event<M>, E>> {
            let mut state = self.0.state.lock();

            loop {
                if let Some(data) = &mut *state {
                    let pulled = mem::replace(data, ConnState(None));

                    match pulled {
                        ConnState(Some(event)) => {
                            self.0.state_changed.notify_all();
                            return Some(event);
                        }
                        ConnState(None) => state = self.0.state_changed.wait(state),
                    }
                } else {
                    return None;
                }
            }
        }
    }

    impl<CV, M, E> ErrorType for Connection<CV, M, E>
    where
        CV: RawCondvar,
        E: Debug,
    {
        type Error = E;
    }

    impl<CV, M, E> crate::mqtt::client::Connection for Connection<CV, M, E>
    where
        CV: RawCondvar,
        E: Debug,
    {
        type Message<'a> = M where Self: 'a;

        fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>> {
            Connection::next(self)
        }
    }
}
