use core::cell::RefCell;
use core::fmt::{Debug, Display, Formatter};
use core::marker::PhantomData;
use core::result::Result;
use core::time::Duration;

extern crate alloc;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;

use crate::event_bus::{self, PinnedEventBus};
use crate::service;
use crate::timer;

#[derive(Debug)]
pub enum Error<E, T>
where
    E: Debug,
    T: Debug,
{
    EventBusError(E),
    TimerError(T),
}

impl<E, T> Display for Error<E, T>
where
    E: Display + Debug + Send + Sync + 'static,
    T: Display + Debug + Send + Sync + 'static,
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
    E: Display + Debug + Send + Sync + 'static,
    T: Display + Debug + Send + Sync + 'static,
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

struct State<C> {
    timers_callbacks: Vec<(TimerId, C)>,
    next_id: TimerId,
}

impl<C> State<C> {
    fn new() -> Self {
        Self {
            timers_callbacks: Vec::new(),
            next_id: 0,
        }
    }

    fn callback_ref(&self, timer_id: TimerId) -> C
    where
        C: Clone,
    {
        self.timers_callbacks
            .iter()
            .find(|(id, _)| *id == timer_id)
            .map(|(_, callback)| callback.clone())
            .unwrap_or_else(|| panic!("Unknown timer ID: {}", timer_id))
    }

    fn add(&mut self, callback: C) -> TimerId {
        if self.next_id == TimerId::max_value() {
            panic!("Timer IDs exhausted");
        }

        let timer_id = self.next_id;
        self.timers_callbacks.push((timer_id, callback));

        self.next_id += 1;

        timer_id
    }

    fn remove(&mut self, timer_id: TimerId) -> C {
        let index = self
            .timers_callbacks
            .iter()
            .enumerate()
            .find(|(_, (id, _))| *id == timer_id)
            .map(|(index, _)| index)
            .unwrap_or_else(|| panic!("Unknown timer ID: {}", timer_id));

        self.timers_callbacks.remove(index).1
    }
}

pub struct Timer<C, T, ER> {
    inner_timer: T,
    id: TimerId,
    state: Rc<RefCell<State<C>>>,
    _error_type: PhantomData<*const ER>,
}

impl<C, T, ER> service::Service for Timer<C, T, ER>
where
    T: timer::Timer,
    ER: Debug + Display + Send + Sync + 'static,
{
    type Error = Error<ER, T::Error>;
}

impl<C, T, ER> timer::Timer for Timer<C, T, ER>
where
    T: timer::Timer,
    ER: Debug + Display + Send + Sync + 'static,
{
    fn start(&mut self) -> Result<(), Self::Error> {
        self.inner_timer.start().map_err(Error::TimerError)
    }

    fn is_scheduled(&self) -> Result<bool, Self::Error> {
        self.inner_timer.is_scheduled().map_err(Error::TimerError)
    }

    fn cancel(&mut self) -> Result<bool, Self::Error> {
        self.inner_timer.cancel().map_err(Error::TimerError)
    }
}

impl<C, T, ER> Drop for Timer<C, T, ER> {
    fn drop(&mut self) {
        self.state.borrow_mut().remove(self.id);
    }
}

pub struct Pinned<C, T, E, P>
where
    E: event_bus::PinnedEventBus<TimerId>,
{
    timer_service: T,
    postbox: P,
    state: Rc<RefCell<State<C>>>,
    _subscription: E::Subscription,
}

pub type Once = Box<dyn FnOnce() + 'static>;
pub type Periodic = Rc<RefCell<dyn FnMut() + 'static>>;

impl<C, T, E, P> service::Service for Pinned<C, T, E, P>
where
    T: service::Service,
    E: PinnedEventBus<TimerId>,
    P: service::Service,
    P::Error: Into<E::Error>,
{
    type Error = Error<E::Error, T::Error>;
}

impl<T, E, P> Pinned<Once, T, E, P>
where
    T: timer::Once,
    E: event_bus::PinnedEventBus<TimerId>,
    P: event_bus::Postbox<TimerId> + Send + Clone,
    P::Error: Into<E::Error>,
{
    pub fn new(
        timer_service: T,
        event_bus: &mut E,
        postbox: P,
    ) -> Result<Self, Error<E::Error, T::Error>> {
        let state = Rc::new(RefCell::new(State::new()));
        let state_subscription = state.clone();

        let subscription = event_bus
            .subscribe(move |timer_id| {
                let callback: Once = state_subscription.borrow_mut().remove(*timer_id);

                (callback)();

                Result::<_, E::Error>::Ok(())
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

impl<T, E, P> timer::PinnedOnce for Pinned<Once, T, E, P>
where
    T: timer::Once,
    E: event_bus::PinnedEventBus<TimerId>,
    P: event_bus::Postbox<TimerId> + Send + Clone + 'static,
    P::Error: Into<E::Error>,
{
    type Timer = Timer<Once, T::Timer, E::Error>;

    fn after<ER>(
        &mut self,
        duration: Duration,
        callback: impl FnOnce() -> Result<(), ER> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        ER: Display + Debug + Sync + Send + 'static,
    {
        let timer_id = self
            .state
            .borrow_mut()
            .add(Box::new(move || (callback)().unwrap()));

        let mut postbox = self.postbox.clone();

        Ok(Timer {
            inner_timer: self
                .timer_service
                .after(duration, move || {
                    postbox
                        .post(timer_id, None)
                        .map(|_| ())
                        .map_err(Error::<_, T::Error>::EventBusError)
                })
                .map_err(Error::TimerError)?,
            id: timer_id,
            state: self.state.clone(),
            _error_type: PhantomData,
        })
    }
}

impl<T, E, P> Pinned<Periodic, T, E, P>
where
    T: timer::Periodic,
    E: event_bus::PinnedEventBus<TimerId>,
    P: event_bus::Postbox<TimerId> + Send + Clone,
    P::Error: Into<E::Error>,
{
    pub fn new(
        timer_service: T,
        event_bus: &mut E,
        postbox: P,
    ) -> Result<Self, Error<E::Error, T::Error>> {
        let state = Rc::new(RefCell::new(State::new()));
        let state_subscription = state.clone();

        let subscription = event_bus
            .subscribe(move |timer_id| {
                let callback: Periodic = state_subscription.borrow().callback_ref(*timer_id);

                (callback.borrow_mut())();

                Result::<_, E::Error>::Ok(())
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

impl<T, E, P> timer::PinnedPeriodic for Pinned<Periodic, T, E, P>
where
    T: timer::Periodic,
    E: event_bus::PinnedEventBus<TimerId>,
    P: event_bus::Postbox<TimerId> + Send + Clone + 'static,
    P::Error: Into<E::Error>,
{
    type Timer = Timer<Periodic, T::Timer, E::Error>;

    fn every<ER>(
        &mut self,
        duration: Duration,
        mut callback: impl FnMut() -> Result<(), ER> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        ER: Display + Debug + Sync + Send + 'static,
    {
        let timer_id = self
            .state
            .borrow_mut()
            .add(Rc::new(RefCell::new(move || (callback)().unwrap())));

        let mut postbox = self.postbox.clone();

        Ok(Timer {
            inner_timer: self
                .timer_service
                .every(duration, move || {
                    postbox
                        .post(timer_id, None)
                        .map(|_| ())
                        .map_err(Error::<_, T::Error>::EventBusError)
                })
                .map_err(Error::TimerError)?,
            id: timer_id,
            state: self.state.clone(),
            _error_type: PhantomData,
        })
    }
}
