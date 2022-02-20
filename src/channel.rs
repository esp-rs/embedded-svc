#[cfg(feature = "experimental")]
pub mod nonblocking {
    use core::future::Future;

    use crate::service::Service;

    pub trait Sender: Service {
        type Data;

        type SendFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_>;
    }

    pub trait Receiver: Service {
        type Data;

        type RecvFuture<'a>: Future<Output = Result<Self::Data, Self::Error>>
        where
            Self: 'a;

        fn recv(&mut self) -> Self::RecvFuture<'_>;
    }

    // TODO: Not clear yet if necessary
    // pub mod adapter {
    //     use core::future::Future;
    //     use core::marker::PhantomData;
    //     use core::pin::Pin;
    //     use core::task::{Context, Poll};

    //     use crate::service::Service;

    //     use super::{Receiver, Sender};

    //     pub fn sender<S, P>(sender: S) -> impl Sender<Data = P>
    //     where
    //         S: Sender,
    //         S::Data: From<P>,
    //     {
    //         SenderAdapter::new(sender)
    //     }

    //     pub fn receiver<R, P>(receiver: R) -> impl Receiver<Data = P>
    //     where
    //         R: Receiver,
    //         R::Data: Into<P>,
    //     {
    //         ReceiverAdapter::new(receiver)
    //     }

    //     struct SenderAdapter<S, P> {
    //         inner_sender: S,
    //         _input: PhantomData<fn() -> P>,
    //     }

    //     impl<S, P> SenderAdapter<S, P> {
    //         pub fn new(inner_sender: S) -> Self
    //         where
    //             S: Sender + Service,
    //             S::Data: From<P>,
    //         {
    //             Self {
    //                 inner_sender,
    //                 _input: PhantomData,
    //             }
    //         }
    //     }

    //     impl<S, P> Service for SenderAdapter<S, P>
    //     where
    //         S: Service,
    //     {
    //         type Error = S::Error;
    //     }

    //     impl<S, P> Sender for SenderAdapter<S, P>
    //     where
    //         S: Sender,
    //         S::Data: From<P>,
    //     {
    //         type Data = P;

    //         type SendFuture<'a>
    //         where
    //             Self: 'a,
    //         = S::SendFuture<'a>;

    //         fn send(&mut self, value: Self::Data) -> Self::SendFuture<'_> {
    //             self.inner_sender.send(value.into())
    //         }
    //     }

    //     struct ReceiverAdapter<R, P> {
    //         inner_receiver: R,
    //         _output: PhantomData<fn() -> P>,
    //     }

    //     impl<R, P> ReceiverAdapter<R, P> {
    //         pub fn new(inner_receiver: R) -> Self
    //         where
    //             R: Receiver + Service,
    //             R::Data: Into<P>,
    //         {
    //             Self {
    //                 inner_receiver,
    //                 _output: PhantomData,
    //             }
    //         }
    //     }

    //     pub struct ReceiverAdapterFuture<'a, R, P>
    //     where
    //         R: Receiver + 'a,
    //     {
    //         inner_future: R::RecvFuture<'a>,
    //         _output: PhantomData<fn() -> P>,
    //     }

    //     impl<'a, R, P> Future for ReceiverAdapterFuture<'a, R, P>
    //     where
    //         R: Receiver,
    //         R::Data: Into<P>,
    //     {
    //         type Output = Result<P, R::Error>;

    //         fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    //             let inner_future = unsafe { self.map_unchecked_mut(|s| &mut s.inner_future) };

    //             match inner_future.poll(cx) {
    //                 Poll::Ready(result) => Poll::Ready(result.map(Into::into)),
    //                 Poll::Pending => Poll::Pending,
    //             }
    //         }
    //     }

    //     impl<R, P> Service for ReceiverAdapter<R, P>
    //     where
    //         R: Service,
    //     {
    //         type Error = R::Error;
    //     }

    //     impl<R, P> Receiver for ReceiverAdapter<R, P>
    //     where
    //         R: Receiver,
    //         R::Data: Into<P>,
    //     {
    //         type Data = P;

    //         type RecvFuture<'a>
    //         where
    //             Self: 'a,
    //         = ReceiverAdapterFuture<'a, R, P>;

    //         fn recv(&mut self) -> Self::RecvFuture<'_> {
    //             ReceiverAdapterFuture {
    //                 inner_future: self.inner_receiver.recv(),
    //                 _output: PhantomData,
    //             }
    //         }
    //     }
    // }
}
