use core::fmt::{Debug, Display};
use core::result::Result;
use core::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Priority {
    VeryHigh,
    High,
    Medium,
    Low,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Medium
    }
}

pub trait Timer<'a> {
    type Error: Display + Debug + Send + Sync + 'static;

    fn callback<E>(
        &mut self,
        callback: Option<impl Fn() -> Result<(), E> + 'a>,
    ) -> Result<(), Self::Error>
    where
        E: Display + Debug;

    fn schedule(&mut self, after: Duration) -> Result<(), Self::Error>;

    fn is_scheduled(&self) -> Result<bool, Self::Error>;

    fn cancel(&mut self) -> Result<bool, Self::Error>;
}

pub trait TimerService<'a> {
    type Error: Display + Debug + Send + Sync + 'static;

    type Timer<'b>: Timer<'b, Error = Self::Error>
    where
        Self: 'b;

    fn timer(
        &self,
        priority: Priority,
        name: impl AsRef<str>,
    ) -> Result<Self::Timer<'a>, Self::Error>;
}
