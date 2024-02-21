use core::fmt::Debug;

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

pub trait Sender: ErrorType {
    type Data<'a>;

    fn send(&mut self, value: Self::Data<'_>) -> Result<(), Self::Error>;
}

impl<S> Sender for &mut S
where
    S: Sender,
{
    type Data<'a> = S::Data<'a>;

    fn send(&mut self, value: Self::Data<'_>) -> Result<(), Self::Error> {
        (**self).send(value)
    }
}

pub trait Receiver: ErrorType {
    type Data<'a>
    where
        Self: 'a;

    fn recv(&mut self) -> Result<Self::Data<'_>, Self::Error>;
}

impl<R> Receiver for &mut R
where
    R: Receiver,
{
    type Data<'a> = R::Data<'a> where Self: 'a;

    fn recv(&mut self) -> Result<Self::Data<'_>, Self::Error> {
        (**self).recv()
    }
}

pub mod asynch {
    pub use super::ErrorType;

    pub trait Sender: ErrorType {
        type Data<'a>: Send;

        async fn send(&mut self, value: Self::Data<'_>) -> Result<(), Self::Error>;
    }

    impl<S> Sender for &mut S
    where
        S: Sender,
    {
        type Data<'a> = S::Data<'a>;

        async fn send(&mut self, value: Self::Data<'_>) -> Result<(), Self::Error> {
            (**self).send(value).await
        }
    }

    pub trait Receiver: ErrorType {
        type Data<'a>
        where
            Self: 'a;

        async fn recv(&mut self) -> Result<Self::Data<'_>, Self::Error>;
    }

    impl<R> Receiver for &mut R
    where
        R: Receiver,
    {
        type Data<'a> = R::Data<'a> where Self: 'a;

        async fn recv(&mut self) -> Result<Self::Data<'_>, Self::Error> {
            (**self).recv().await
        }
    }
}
