use core::fmt::Debug;
use core::result::Result;
use core::time::Duration;

pub trait ErrorType {
    type Error: Debug;
}

impl<E> ErrorType for &E
where
    E: ErrorType,
{
    type Error = E::Error;
}

impl<E> ErrorType for &mut E
where
    E: ErrorType,
{
    type Error = E::Error;
}

pub trait Spin: ErrorType {
    fn spin(&mut self, duration: Option<Duration>) -> Result<(), Self::Error>;
}

pub trait Postbox<P>: ErrorType {
    fn post(&self, payload: &P, wait: Option<Duration>) -> Result<bool, Self::Error>;
}

impl<'a, P, PB> Postbox<P> for &'a mut PB
where
    PB: Postbox<P> + ErrorType,
{
    fn post(&self, payload: &P, wait: Option<Duration>) -> Result<bool, Self::Error> {
        (**self).post(payload, wait)
    }
}

impl<'a, P, PB> Postbox<P> for &'a PB
where
    PB: Postbox<P> + ErrorType,
{
    fn post(&self, payload: &P, wait: Option<Duration>) -> Result<bool, Self::Error> {
        (*self).post(payload, wait)
    }
}

pub trait EventBus<P>: ErrorType {
    type Subscription;

    fn subscribe(
        &self,
        callback: impl for<'a> FnMut(&'a P) + Send + 'static,
    ) -> Result<Self::Subscription, Self::Error>;
}

impl<'a, P, E> EventBus<P> for &'a mut E
where
    E: EventBus<P>,
{
    type Subscription = E::Subscription;

    fn subscribe(
        &self,
        callback: impl for<'b> FnMut(&'b P) + Send + 'static,
    ) -> Result<Self::Subscription, Self::Error> {
        (**self).subscribe(callback)
    }
}

impl<'a, P, E> EventBus<P> for &'a E
where
    E: EventBus<P>,
{
    type Subscription = E::Subscription;

    fn subscribe(
        &self,
        callback: impl for<'b> FnMut(&'b P) + Send + 'static,
    ) -> Result<Self::Subscription, Self::Error> {
        (*self).subscribe(callback)
    }
}

pub trait PostboxProvider<P>: ErrorType {
    type Postbox: Postbox<P, Error = Self::Error>;

    fn postbox(&self) -> Result<Self::Postbox, Self::Error>;
}

impl<'a, P, PP> PostboxProvider<P> for &'a mut PP
where
    PP: PostboxProvider<P>,
{
    type Postbox = PP::Postbox;

    fn postbox(&self) -> Result<Self::Postbox, Self::Error> {
        (**self).postbox()
    }
}

impl<'a, P, PP> PostboxProvider<P> for &'a PP
where
    PP: PostboxProvider<P>,
{
    type Postbox = PP::Postbox;

    fn postbox(&self) -> Result<Self::Postbox, Self::Error> {
        (*self).postbox()
    }
}

#[cfg(feature = "nightly")]
pub mod asynch {
    pub use super::{ErrorType, Spin};

    pub trait Sender {
        type Data: Send;
        type Result: Send;

        async fn send(&self, value: Self::Data) -> Self::Result;
    }

    impl<S> Sender for &mut S
    where
        S: Sender,
    {
        type Data = S::Data;
        type Result = S::Result;

        async fn send(&self, value: Self::Data) -> Self::Result {
            (**self).send(value).await
        }
    }

    impl<S> Sender for &S
    where
        S: Sender,
    {
        type Data = S::Data;
        type Result = S::Result;

        async fn send(&self, value: Self::Data) -> Self::Result {
            (*self).send(value).await
        }
    }

    pub trait Receiver {
        type Result: Send;

        async fn recv(&self) -> Self::Result;
    }

    impl<R> Receiver for &mut R
    where
        R: Receiver,
    {
        type Result = R::Result;

        async fn recv(&self) -> Self::Result {
            (**self).recv().await
        }
    }

    impl<R> Receiver for &R
    where
        R: Receiver,
    {
        type Result = R::Result;

        async fn recv(&self) -> Self::Result {
            (*self).recv().await
        }
    }

    pub trait EventBus<P>: ErrorType {
        type Subscription: Receiver<Result = P>;

        fn subscribe(&self) -> Result<Self::Subscription, Self::Error>;
    }

    impl<E, P> EventBus<P> for &mut E
    where
        E: EventBus<P>,
    {
        type Subscription = E::Subscription;

        fn subscribe(&self) -> Result<Self::Subscription, Self::Error> {
            (**self).subscribe()
        }
    }

    impl<E, P> EventBus<P> for &E
    where
        E: EventBus<P>,
    {
        type Subscription = E::Subscription;

        fn subscribe(&self) -> Result<Self::Subscription, Self::Error> {
            (**self).subscribe()
        }
    }

    pub trait PostboxProvider<P>: ErrorType {
        type Postbox: Sender<Data = P>;

        fn postbox(&self) -> Result<Self::Postbox, Self::Error>;
    }

    impl<PB, P> PostboxProvider<P> for &mut PB
    where
        PB: PostboxProvider<P>,
    {
        type Postbox = PB::Postbox;

        fn postbox(&self) -> Result<Self::Postbox, Self::Error> {
            (**self).postbox()
        }
    }

    impl<PB, P> PostboxProvider<P> for &PB
    where
        PB: PostboxProvider<P>,
    {
        type Postbox = PB::Postbox;

        fn postbox(&self) -> Result<Self::Postbox, Self::Error> {
            (**self).postbox()
        }
    }
}
