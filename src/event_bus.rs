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
    type Subscription<'a>
    where
        Self: 'a;

    fn subscribe<F>(&self, callback: F) -> Result<Self::Subscription<'_>, Self::Error>
    where
        F: FnMut(&P) + Send + 'static;
}

impl<'e, P, E> EventBus<P> for &'e E
where
    E: EventBus<P>,
{
    type Subscription<'a> = E::Subscription<'a> where Self: 'a;

    fn subscribe<F>(&self, callback: F) -> Result<Self::Subscription<'_>, Self::Error>
    where
        F: FnMut(&P) + Send + 'static,
    {
        (**self).subscribe(callback)
    }
}

impl<'e, P, E> EventBus<P> for &'e mut E
where
    E: EventBus<P>,
{
    type Subscription<'a> = E::Subscription<'a> where Self: 'a;

    fn subscribe<F>(&self, callback: F) -> Result<Self::Subscription<'_>, Self::Error>
    where
        F: FnMut(&P) + Send + 'static,
    {
        (**self).subscribe(callback)
    }
}

pub trait PostboxProvider<P>: ErrorType {
    type Postbox<'a>: Postbox<P, Error = Self::Error>
    where
        Self: 'a;

    fn postbox(&self) -> Result<Self::Postbox<'_>, Self::Error>;
}

impl<'p, P, PP> PostboxProvider<P> for &'p mut PP
where
    PP: PostboxProvider<P>,
{
    type Postbox<'a> = PP::Postbox<'a> where Self: 'a;

    fn postbox(&self) -> Result<Self::Postbox<'_>, Self::Error> {
        (**self).postbox()
    }
}

impl<'p, P, PP> PostboxProvider<P> for &'p PP
where
    PP: PostboxProvider<P>,
{
    type Postbox<'a> = PP::Postbox<'a> where Self: 'a;

    fn postbox(&self) -> Result<Self::Postbox<'_>, Self::Error> {
        (*self).postbox()
    }
}

pub mod asynch {
    pub use super::{ErrorType, Spin};

    pub trait Sender: ErrorType {
        type Data: Send;

        async fn send(&mut self, value: Self::Data) -> Result<(), Self::Error>;
    }

    impl<S> Sender for &mut S
    where
        S: Sender,
    {
        type Data = S::Data;

        async fn send(&mut self, value: Self::Data) -> Result<(), Self::Error> {
            (**self).send(value).await
        }
    }

    pub trait Receiver: ErrorType {
        type Data: Send;

        async fn recv(&mut self) -> Result<Self::Data, Self::Error>;
    }

    impl<R> Receiver for &mut R
    where
        R: Receiver,
    {
        type Data = R::Data;

        async fn recv(&mut self) -> Result<Self::Data, Self::Error> {
            (**self).recv().await
        }
    }

    pub trait EventBus<P>: ErrorType {
        type Subscription<'a>: Receiver<Data = P, Error = Self::Error>
        where
            Self: 'a;

        async fn subscribe(&self) -> Result<Self::Subscription<'_>, Self::Error>;
    }

    impl<E, P> EventBus<P> for &mut E
    where
        E: EventBus<P>,
    {
        type Subscription<'a> = E::Subscription<'a> where Self: 'a;

        async fn subscribe(&self) -> Result<Self::Subscription<'_>, Self::Error> {
            (**self).subscribe().await
        }
    }

    impl<E, P> EventBus<P> for &E
    where
        E: EventBus<P>,
    {
        type Subscription<'a> = E::Subscription<'a> where Self: 'a;

        async fn subscribe(&self) -> Result<Self::Subscription<'_>, Self::Error> {
            (**self).subscribe().await
        }
    }

    pub trait PostboxProvider<P>: ErrorType {
        type Postbox<'a>: Sender<Data = P>
        where
            Self: 'a;

        async fn postbox(&self) -> Result<Self::Postbox<'_>, Self::Error>;
    }

    impl<PB, P> PostboxProvider<P> for &mut PB
    where
        PB: PostboxProvider<P>,
    {
        type Postbox<'a> = PB::Postbox<'a> where Self: 'a;

        async fn postbox(&self) -> Result<Self::Postbox<'_>, Self::Error> {
            (**self).postbox().await
        }
    }

    impl<PB, P> PostboxProvider<P> for &PB
    where
        PB: PostboxProvider<P>,
    {
        type Postbox<'a> = PB::Postbox<'a> where Self: 'a;

        async fn postbox(&self) -> Result<Self::Postbox<'_>, Self::Error> {
            (**self).postbox().await
        }
    }
}
