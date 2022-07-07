use core::fmt::{self, Debug, Display, Write as _};

use crate::io::{Io, Read, Write};

pub use super::{Headers, Query, RequestId, SendHeaders, SendStatus};

pub mod middleware;
pub mod registry;

pub trait Request: Query + RequestId + Headers + Read {
    type Headers: Query + RequestId + Headers;

    type Body: Read<Error = Self::Error>;

    fn split(self) -> (Self::Headers, Self::Body);
}

pub trait Response<const B: usize = 64>: SendStatus + SendHeaders + Io {
    type Write: Write<Error = Self::Error>;

    fn into_writer(self) -> Result<Self::Write, Self::Error>
    where
        Self: Sized;
}

pub struct HandlerError(heapless::String<128>);

impl<E> From<E> for HandlerError
where
    E: Debug,
{
    fn from(e: E) -> Self {
        let mut string: heapless::String<128> = "".into();

        if write!(&mut string, "{:?}", e).is_err() {
            string = "(Error string too big to serve)".into();
        }

        Self(string)
    }
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type HandlerResult = Result<(), HandlerError>;

pub trait Handler<R, S>: Send
where
    R: Request,
    S: Response,
{
    fn handle(&self, req: R, resp: S) -> HandlerResult;
}

impl<R, S, H> Handler<R, S> for &H
where
    R: Request,
    S: Response,
    H: Handler<R, S> + Send + Sync,
{
    fn handle(&self, req: R, resp: S) -> HandlerResult {
        (*self).handle(req, resp)
    }
}

pub struct FnHandler<F>(F);

impl<F> FnHandler<F> {
    pub const fn new<R, S>(f: F) -> Self
    where
        R: Request,
        S: Response,
        F: Fn(R, S) -> HandlerResult,
    {
        Self(f)
    }
}

impl<R, S, F> Handler<R, S> for FnHandler<F>
where
    R: Request,
    S: Response,
    F: Fn(R, S) -> HandlerResult + Send,
{
    fn handle(&self, req: R, resp: S) -> HandlerResult {
        self.0(req, resp)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::io::{asynch::Read, asynch::Write, Io};
    use crate::unblocker::asynch::{Blocker, Blocking, TrivialAsync};

    pub use crate::http::{Headers, Query, RequestId, SendHeaders, SendStatus};

    pub use super::{HandlerError, HandlerResult};

    pub trait Request: Query + RequestId + Headers + Read {
        type Headers: Query + RequestId + Headers;

        type Body: Read<Error = Self::Error>;

        fn split(self) -> (Self::Headers, Self::Body);
    }

    pub trait Response: SendStatus + SendHeaders + Io {
        type Write: Write<Error = Self::Error>;

        type IntoWriterFuture: Future<Output = Result<Self::Write, Self::Error>>;

        fn into_writer(self) -> Self::IntoWriterFuture
        where
            Self: Sized;
    }

    pub trait Handler<R, S>: Send
    where
        R: Request,
        S: Response,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult>
        where
            Self: 'a;

        fn handle(&self, req: R, resp: S) -> Self::HandleFuture<'_>;
    }

    impl<R, S, H> Handler<R, S> for &H
    where
        R: Request,
        S: Response,
        H: Handler<R, S> + Send + Sync,
    {
        type HandleFuture<'a>
        where
            Self: 'a,
        = H::HandleFuture<'a>;

        fn handle(&self, req: R, resp: S) -> Self::HandleFuture<'_> {
            (*self).handle(req, resp)
        }
    }

    impl<B, R> super::Request for Blocking<B, R>
    where
        B: Blocker,
        R: Request,
    {
        type Headers = R::Headers;

        type Body = Blocking<B, R::Body>;

        fn split(self) -> (Self::Headers, Self::Body) {
            let (headers, body) = self.1.split();

            (headers, Blocking::new(self.0, body))
        }
    }

    impl<B, R> super::Response for Blocking<B, R>
    where
        B: Blocker,
        R: Response,
    {
        type Write = Blocking<B, R::Write>;

        fn into_writer(self) -> Result<Self::Write, Self::Error>
        where
            Self: Sized,
        {
            let response = self.0.block_on(self.1.into_writer())?;

            Ok(Blocking::new(self.0, response))
        }
    }

    // Implement a blocking handler on top of an async handler
    // (use case: user provides us an async handler, but we are a blocking server)
    impl<B, H, R, S> super::Handler<R, S> for Blocking<B, H>
    where
        B: Blocker + Send,
        H: Handler<TrivialAsync<R>, TrivialAsync<S>>,
        R: super::Request,
        S: super::Response,
    {
        fn handle(&self, req: R, resp: S) -> HandlerResult {
            self.0.block_on(
                self.1
                    .handle(TrivialAsync::new_async(req), TrivialAsync::new_async(resp)),
            )
        }
    }

    impl<R> Request for TrivialAsync<R>
    where
        R: super::Request,
    {
        type Headers = R::Headers;

        type Body = TrivialAsync<R::Body>;

        fn split(self) -> (Self::Headers, Self::Body) {
            let (headers, body) = self.1.split();

            (headers, TrivialAsync::new_async(body))
        }
    }

    impl<R> Response for TrivialAsync<R>
    where
        R: super::Response,
    {
        type Write = TrivialAsync<R::Write>;

        type IntoWriterFuture = impl Future<Output = Result<Self::Write, Self::Error>>;

        fn into_writer(self) -> Self::IntoWriterFuture
        where
            Self: Sized,
        {
            async move { self.1.into_writer().map(TrivialAsync::new_async) }
        }
    }

    // Implement an async handler on top of a blocking handler
    // (use case: user provides us a blocking handler, but we are an async server)
    impl<B, H, R, S> Handler<R, S> for Blocking<B, H>
    where
        B: Blocker + Clone + Send,
        H: super::Handler<Blocking<B, R>, Blocking<B, S>>,
        R: Request,
        S: Response,
    {
        type HandleFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = HandlerResult>;

        fn handle(&self, req: R, resp: S) -> Self::HandleFuture<'_> {
            async move {
                self.1.handle(
                    Blocking::new(self.0.clone(), req),
                    Blocking::new(self.0.clone(), resp),
                )
            }
        }
    }

    // type HFuture = impl Future<Output = HandlerResult;

    // impl<R, S, H> Handler<R, S> for H
    // where
    //     R: Request,
    //     S: Response,
    //     H: Fn(R, S) -> HFuture + 'static,
    // {
    //     type HandleFuture<'a> where Self: 'a = HFuture;

    //     fn handle(&mut self, req: R, resp: S) -> Self::HandleFuture<'_> {
    //         (self)(req, resp)
    //     }
    // }
}
