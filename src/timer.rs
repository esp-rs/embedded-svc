use core::fmt::{Debug, Display};
use core::result::Result;
use core::time::Duration;

#[derive(Default, Clone, Debug)]
pub struct TimerConfiguration<'a> {
    pub name: Option<&'a str>,
    pub skip_unhandled_events: bool,
}

pub trait Timer {
    type Error: Display + Debug + Send + Sync + 'static;

    fn once(&mut self, after: Duration) -> Result<(), Self::Error>;

    fn periodic(&mut self, after: Duration) -> Result<(), Self::Error>;

    fn is_scheduled(&self) -> Result<bool, Self::Error>;

    fn cancel(&mut self) -> Result<bool, Self::Error>;
}

pub trait TimerService {
    type Error: Display + Debug + Send + Sync + 'static;

    type Timer: Timer<Error = Self::Error> + 'static;

    fn timer<E>(
        &self,
        conf: &TimerConfiguration<'_>,
        callback: impl FnMut() -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}

pub trait PinnedTimerService {
    type Error: Display + Debug + Send + Sync + 'static;

    type Timer: Timer<Error = Self::Error> + 'static;

    fn timer<E>(
        &self,
        conf: &TimerConfiguration<'_>,
        callback: impl FnMut() -> Result<(), E> + 'static,
    ) -> Result<Self::Timer, Self::Error>
    where
        E: Display + Debug + Sync + Send + 'static;
}
