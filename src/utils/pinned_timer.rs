use core::fmt::{Debug, Display, Formatter};
use core::marker::PhantomData;
use core::result::Result;
use std::cell::RefCell;
use std::rc::Rc;

extern crate alloc;
use alloc::vec::Vec;

use crate::event_bus;
use crate::timer;

#[derive(Debug)]
pub enum Error<E, T>
where
    E: Display + Debug,
    T: Display + Debug,
{
    EventBusError(E),
    TimerError(T),
}

impl<E, T> Display for Error<E, T>
where
    E: Display + Debug,
    T: Display + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::EventBusError(s) => write!(f, "Event Bus Error {}", s),
            Error::TimerError(w) => write!(f, "Timer Error {}", w),
        }
    }
}

#[cfg(feature = "std")]
impl<E, T> std::error::Error for Error<E, T>
where
    E: Display + Debug,
    T: Display + Debug,
    // TODO
    // where
    //     S: std::error::Error + 'static,
    //     W: std::error::Error + 'static,
{
    // TODO
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     match self {
    //         EventBusTimerError::EventBusError(s) => Some(s),
    //         EventBusTimerError::TimerError(w) => Some(w),
    //     }
    // }
}

type TimerId = u32;

struct State {
    timers_callbacks: Vec<(TimerId, Rc<RefCell<dyn FnMut() + 'static>>)>,
    next_id: TimerId,
}

impl State {
    fn new() -> Self {
        Self {
            timers_callbacks: Vec::new(),
            next_id: 0,
        }
    }

    fn call(&self, timer_id: TimerId) {
        let callback = self
            .timers_callbacks
            .iter()
            .find(|(id, _)| *id == timer_id)
            .map(|(_, callback)| callback.clone());

        if let Some(callback) = callback {
            (callback.borrow_mut())();
        }
    }

    fn add(&mut self, callback: Rc<RefCell<dyn FnMut() + 'static>>) -> TimerId {
        if self.next_id == TimerId::max_value() {
            panic!("Timer IDs exhausted");
        }

        let timer_id = self.next_id;
        self.timers_callbacks.push((timer_id, callback));

        self.next_id += 1;

        timer_id
    }

    fn remove(&mut self, timer_id: TimerId) {
        let index = self
            .timers_callbacks
            .iter()
            .enumerate()
            .find(|(_, (id, _))| *id == timer_id)
            .map(|(index, _)| index)
            .unwrap_or_else(|| panic!("Unknown timer ID: {}", timer_id));

        self.timers_callbacks.remove(index);
    }
}

pub struct Timer<T, ER> {
    inner_timer: T,
    id: TimerId,
    state: Rc<RefCell<State>>,
    _error_type: PhantomData<*const ER>,
}

impl<T, ER> timer::Timer for Timer<T, ER>
where
    T: timer::Timer,
    ER: Debug + Display + Send + Sync + 'static,
{
    type Error = Error<ER, T::Error>;

    fn once(&mut self, after: std::time::Duration) -> Result<(), Self::Error> {
        self.inner_timer.once(after).map_err(Error::TimerError)
    }

    fn periodic(&mut self, after: std::time::Duration) -> Result<(), Self::Error> {
        self.inner_timer.periodic(after).map_err(Error::TimerError)
    }

    fn is_scheduled(&self) -> Result<bool, Self::Error> {
        self.inner_timer.is_scheduled().map_err(Error::TimerError)
    }

    fn cancel(&mut self) -> Result<bool, Self::Error> {
        self.inner_timer.cancel().map_err(Error::TimerError)
    }
}

impl<T, ER> Drop for Timer<T, ER> {
    fn drop(&mut self) {
        self.state.borrow_mut().remove(self.id);
    }
}

pub struct PinnedTimerService<T, E, P>
where
    E: event_bus::PinnedEventBus,
{
    timer_service: T,
    postbox: P,
    state: Rc<RefCell<State>>,
    _subscription: E::Subscription<TimerId>,
}

impl<T, E, P> PinnedTimerService<T, E, P>
where
    T: timer::TimerService,
    E: event_bus::PinnedEventBus,
    P: event_bus::Postbox + Send + Clone,
    P::Error: Into<E::Error>,
{
    const EVENT_SOURCE: event_bus::Source<TimerId> =
        event_bus::Source::new(b"PINNED_TIMER_SERVICE\0");

    pub fn new(
        timer_service: T,
        event_bus: &E,
        postbox: P,
    ) -> Result<Self, Error<E::Error, T::Error>> {
        let state = Rc::new(RefCell::new(State::new()));
        let state_subscription = state.clone();

        let subscription = event_bus
            .subscribe(Self::EVENT_SOURCE, move |timer_id| {
                Result::<_, E::Error>::Ok(state_subscription.borrow().call(*timer_id))
            })
            .map_err(Error::EventBusError)?;

        Ok(Self {
            timer_service,
            postbox,
            state,
            _subscription: subscription,
        })
    }
}

impl<T, E, P> timer::PinnedTimerService for PinnedTimerService<T, E, P>
where
    T: timer::TimerService,
    E: event_bus::PinnedEventBus,
    P: event_bus::Postbox + Send + Clone + 'static,
    P::Error: Into<E::Error>,
{
    type Error = Error<E::Error, T::Error>;

    type Timer = Timer<T::Timer, E::Error>;

    fn timer<ER>(
        &self,
        conf: &timer::TimerConfiguration<'_>,
        mut callback: impl FnMut() -> Result<(), ER> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        ER: Display + Debug + Sync + Send + 'static,
    {
        let timer_id = self
            .state
            .borrow_mut()
            .add(Rc::new(RefCell::new(move || (callback)().unwrap())));

        let postbox = self.postbox.clone();

        Ok(Timer {
            inner_timer: self
                .timer_service
                .timer(conf, move || {
                    postbox
                        .post(&Self::EVENT_SOURCE, &timer_id)
                        .map_err(Error::<_, T::Error>::EventBusError)
                })
                .map_err(Error::TimerError)?,
            id: timer_id,
            state: self.state.clone(),
            _error_type: PhantomData,
        })
    }
}
