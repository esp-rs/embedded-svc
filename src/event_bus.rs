use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use core::result::Result;
use core::time::Duration;

use crate::service::Service;

#[derive(Copy, Clone)]
pub struct Source<P> {
    id: &'static [u8],
    _payload_meta: PhantomData<*const P>,
}

impl<P> Source<P> {
    pub const fn new(id: &'static [u8]) -> Self {
        Self {
            id,
            _payload_meta: PhantomData,
        }
    }

    pub fn id(&self) -> &'static [u8] {
        self.id
    }
}

unsafe impl<P> Send for Source<P> {}
unsafe impl<P> Sync for Source<P> {}

pub trait Subscription<P> {}

pub trait Postbox: Service {
    fn post<P>(&self, source: &Source<P>, payload: &P) -> Result<(), Self::Error>
    where
        P: Copy;
}

pub trait Spin: Service {
    fn spin(&self, duration: Option<Duration>) -> Result<(), Self::Error>;
}

pub trait EventBus: Postbox {
    type Subscription<P>: Subscription<P> + Send;

    fn subscribe<P, E>(
        &self,
        source: Source<P>,
        callback: impl for<'a> FnMut(&'a P) -> Result<(), E> + Send + 'static,
    ) -> Result<Self::Subscription<P>, Self::Error>
    where
        P: Copy + Send + Sync + 'static,
        E: Display + Debug + Send + Sync + 'static;
}

pub trait PinnedEventBus: Service {
    type Subscription<P>: Subscription<P>;

    fn subscribe<P, E>(
        &self,
        source: Source<P>,
        callback: impl for<'a> FnMut(&'a P) -> Result<(), E> + 'static,
    ) -> Result<Self::Subscription<P>, Self::Error>
    where
        P: Copy + Send + Sync + 'static,
        E: Display + Debug + Send + Sync + 'static;
}

pub mod nonblocking {
    use core::fmt::{Debug, Display};
    use core::stream::Stream;

    pub use super::{Postbox, Source};

    pub trait EventBus: Postbox {
        type SubscriptionStream<P>: Stream<Item = Result<P, Self::Error>>
        where
            P: Copy;

        fn subscribe<P, E>(
            &self,
            source: Source<P>,
        ) -> Result<Self::SubscriptionStream<P>, Self::Error>
        where
            P: Copy + Send + Sync + 'static,
            E: Display + Debug + Send + Sync + 'static;
    }
}
