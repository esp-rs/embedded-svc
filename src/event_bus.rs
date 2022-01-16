use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use core::result::Result;

pub use crate::timer::Priority;
pub use crate::timer::Timer;
pub use crate::timer::TimerService;

pub struct Source<P> {
    id: &'static str,
    _payload: PhantomData<P>,
}

impl<P> Source<P> {
    pub const fn new(id: &'static str) -> Self {
        Self {
            id,
            _payload: PhantomData,
        }
    }

    pub fn id(&self) -> &'static str {
        self.id
    }
}

pub trait Subscription<'a, P> {
    type Error: Display + Debug + Send + Sync + 'static;

    fn callback<E>(
        &mut self,
        callback: Option<impl for<'b> Fn(&'b P) -> Result<(), E> + 'a>,
    ) -> Result<(), Self::Error>
    where
        E: Display + Debug;
}

pub trait Poster {
    type Error: Display + Debug + Send + Sync + 'static;

    fn post<P>(
        &self,
        priority: Priority,
        source: &Source<P>,
        payload: &P,
    ) -> Result<(), Self::Error>
    where
        P: Copy;
}

pub trait EventBus<'a>:
    Poster<Error = <Self as EventBus<'a>>::Error>
    + TimerService<'a, Error = <Self as EventBus<'a>>::Error>
{
    type Error: Display + Debug + Send + Sync + 'static;

    type Subscription<'b, P>: Subscription<'b, P, Error = <Self as EventBus<'a>>::Error>
    where
        P: Clone,
        Self: 'b;

    fn subscribe<P>(
        &self,
        source: Source<P>,
    ) -> Result<Self::Subscription<'a, P>, <Self as EventBus<'a>>::Error>
    where
        P: Clone;
}
