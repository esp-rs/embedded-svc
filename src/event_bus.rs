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

    use core::future::Future;

    pub trait Sender {
        type Data: Send;

        type SendFuture<'a>: Future + Send
        where
            Self: 'a;

        fn send(&self, value: Self::Data) -> Self::SendFuture<'_>;
    }

    impl<S> Sender for &mut S
    where
        S: Sender,
    {
        type Data = S::Data;

        type SendFuture<'a>
        = S::SendFuture<'a> where Self: 'a;

        fn send(&self, value: Self::Data) -> Self::SendFuture<'_> {
            (**self).send(value)
        }
    }

    impl<S> Sender for &S
    where
        S: Sender,
    {
        type Data = S::Data;

        type SendFuture<'a>
        = S::SendFuture<'a> where Self: 'a;

        fn send(&self, value: Self::Data) -> Self::SendFuture<'_> {
            (*self).send(value)
        }
    }

    pub trait Receiver {
        type Data: Send;

        type RecvFuture<'a>: Future<Output = Self::Data> + Send
        where
            Self: 'a;

        fn recv(&self) -> Self::RecvFuture<'_>;
    }

    impl<R> Receiver for &mut R
    where
        R: Receiver,
    {
        type Data = R::Data;

        type RecvFuture<'a>
        = R::RecvFuture<'a> where Self: 'a;

        fn recv(&self) -> Self::RecvFuture<'_> {
            (**self).recv()
        }
    }

    impl<R> Receiver for &R
    where
        R: Receiver,
    {
        type Data = R::Data;

        type RecvFuture<'a>
        = R::RecvFuture<'a> where Self: 'a;

        fn recv(&self) -> Self::RecvFuture<'_> {
            (*self).recv()
        }
    }

    pub trait EventBus<P>: ErrorType {
        type Subscription: Receiver<Data = P>;

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
