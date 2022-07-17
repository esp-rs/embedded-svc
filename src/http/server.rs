use core::{
    fmt::{self, Debug, Display, Write as _},
    iter,
};

use crate::io::{Read, Write};

pub use super::{Headers, Method, Query, Status};

pub trait Request: Query + Headers + Read {
    type Headers<'b>: Query + Headers
    where
        Self: 'b;
    type Read<'b>: Read<Error = Self::Error>
    where
        Self: 'b;

    type ResponseWrite: Write<Error = Self::Error>;

    fn split<'b>(&'b mut self) -> (Self::Headers<'b>, Self::Read<'b>);

    fn into_response<'a, H>(
        self,
        status: u16,
        message: Option<&'a str>,
        headers: H,
    ) -> Result<Self::ResponseWrite, Self::Error>
    where
        H: IntoIterator<Item = (&'a str, &'a str)>,
        Self: Sized;

    fn into_status_response(self, status: u16) -> Result<Self::ResponseWrite, Self::Error>
    where
        Self: Sized,
    {
        self.into_response(status, None, iter::empty())
    }

    fn into_ok_response(self) -> Result<Self::ResponseWrite, Self::Error>
    where
        Self: Sized,
    {
        self.into_response(200, Some("OK"), iter::empty())
    }
}

pub struct HandlerError(heapless::String<128>);

impl HandlerError {
    pub fn new(message: &str) -> Self {
        Self(message.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

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

pub trait Handler<R>: Send
where
    R: Request,
{
    fn handle(&self, request: R) -> HandlerResult;
}

impl<R, H> Handler<R> for &H
where
    R: Request,
    H: Handler<R> + Send + Sync,
{
    fn handle(&self, request: R) -> HandlerResult {
        (*self).handle(request)
    }
}

pub struct FnHandler<F>(F);

impl<F> FnHandler<F> {
    pub const fn new<R>(f: F) -> Self
    where
        R: Request,
        F: Fn(R) -> HandlerResult,
    {
        Self(f)
    }
}

impl<R, F> Handler<R> for FnHandler<F>
where
    R: Request,
    F: Fn(R) -> HandlerResult + Send,
{
    fn handle(&self, request: R) -> HandlerResult {
        self.0(request)
    }
}

pub trait Middleware<R>: Send
where
    R: Request,
{
    fn handle<H>(&self, request: R, handler: &H) -> HandlerResult
    where
        H: Handler<R>;

    fn compose<H>(self, handler: H) -> CompositeHandler<Self, H>
    where
        H: Handler<R>,
        Self: Sized,
    {
        CompositeHandler::new(self, handler)
    }
}

pub struct CompositeHandler<M, H> {
    middleware: M,
    handler: H,
}

impl<M, H> CompositeHandler<M, H> {
    pub fn new(middleware: M, handler: H) -> Self {
        Self {
            middleware,
            handler,
        }
    }
}

impl<M, H, R> Handler<R> for CompositeHandler<M, H>
where
    M: Middleware<R>,
    H: Handler<R>,
    R: Request,
{
    fn handle(&self, request: R) -> HandlerResult {
        self.middleware.handle(request, &self.handler)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::io::{asynch::Read, asynch::Write};
    use crate::unblocker::asynch::{Blocker, Blocking, TrivialAsync};

    pub use super::{HandlerError, HandlerResult, Headers, Method, Query, Status};

    pub trait Request: Query + Headers + Read {
        type Headers<'b>: Query + Headers
        where
            Self: 'b;
        type Read<'b>: Read<Error = Self::Error>
        where
            Self: 'b;

        type ResponseWrite: Write<Error = Self::Error>;

        type IntoResponseFuture<'a, H>: Future<Output = Result<Self::ResponseWrite, Self::Error>>;
        type IntoOkResponseFuture: Future<Output = Result<Self::ResponseWrite, Self::Error>>;

        fn split<'b>(&'b mut self) -> (Self::Headers<'b>, Self::Read<'b>);

        fn into_response<'a, H>(
            self,
            status: u16,
            message: Option<&'a str>,
            headers: H,
        ) -> Self::IntoResponseFuture<'a, H>
        where
            H: IntoIterator<Item = (&'a str, &'a str)>,
            Self: Sized;

        fn into_ok_response(self) -> Self::IntoOkResponseFuture
        where
            Self: Sized;
    }

    pub trait Handler<R>: Send
    where
        R: Request,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult>
        where
            Self: 'a;

        fn handle(&self, request: R) -> Self::HandleFuture<'_>;
    }

    impl<H, R> Handler<R> for &H
    where
        R: Request,
        H: Handler<R> + Send + Sync,
    {
        type HandleFuture<'a>
        where
            Self: 'a,
        = H::HandleFuture<'a>;

        fn handle(&self, request: R) -> Self::HandleFuture<'_> {
            (*self).handle(request)
        }
    }

    impl<B, R> super::Request for Blocking<B, R>
    where
        B: Blocker + Clone,
        R: Request,
    {
        type Headers<'b>
        where
            Self: 'b,
        = R::Headers<'b>;
        type Read<'b>
        where
            Self: 'b,
        = Blocking<B, R::Read<'b>>;

        type ResponseWrite = Blocking<B, R::ResponseWrite>;

        fn split<'b>(&'b mut self) -> (Self::Headers<'b>, Self::Read<'b>) {
            let (headers, body) = self.1.split();

            (headers, Blocking::new(self.0.clone(), body))
        }

        fn into_response<'a, H>(
            self,
            status: u16,
            message: Option<&'a str>,
            headers: H,
        ) -> Result<Self::ResponseWrite, Self::Error>
        where
            H: IntoIterator<Item = (&'a str, &'a str)>,
            Self: Sized,
        {
            let response = self
                .0
                .block_on(self.1.into_response(status, message, headers))?;

            Ok(Blocking::new(self.0, response))
        }

        fn into_ok_response(self) -> Result<Self::ResponseWrite, Self::Error>
        where
            Self: Sized,
        {
            let response = self.0.block_on(self.1.into_ok_response())?;

            Ok(Blocking::new(self.0, response))
        }
    }

    // // Implement a blocking handler on top of an async handler
    // // (use case: user provides us an async handler, but we are a blocking server)
    // impl<B, H, R> super::Handler<R> for Blocking<B, H>
    // where
    //     B: Blocker + Send,
    //     H: Handler<TrivialAsync<R>>,
    //     R: super::Request,
    // {
    //     fn handle(&self, request: R) -> HandlerResult {
    //         self.0.block_on(self.1.handle(TrivialAsync::new_async(request)))
    //     }
    // }

    impl<R> Request for TrivialAsync<R>
    where
        R: super::Request,
    {
        type Headers<'b>
        where
            Self: 'b,
        = R::Headers<'b>;
        type Read<'b>
        where
            Self: 'b,
        = TrivialAsync<R::Read<'b>>;

        type ResponseWrite = TrivialAsync<R::ResponseWrite>;

        type IntoResponseFuture<'a, H> =
            impl Future<Output = Result<Self::ResponseWrite, Self::Error>>;
        type IntoOkResponseFuture = impl Future<Output = Result<Self::ResponseWrite, Self::Error>>;

        fn split<'b>(&'b mut self) -> (Self::Headers<'b>, Self::Read<'b>) {
            let (headers, body) = self.1.split();

            (headers, TrivialAsync::new_async(body))
        }

        fn into_response<'a, H>(
            self,
            status: u16,
            message: Option<&'a str>,
            headers: H,
        ) -> Self::IntoResponseFuture<'a, H>
        where
            H: IntoIterator<Item = (&'a str, &'a str)>,
            Self: Sized,
        {
            async move {
                Ok(TrivialAsync::new_async(
                    self.1.into_response(status, message, headers)?,
                ))
            }
        }

        fn into_ok_response(self) -> Self::IntoOkResponseFuture
        where
            Self: Sized,
        {
            async move { Ok(TrivialAsync::new_async(self.1.into_ok_response()?)) }
        }
    }

    // // Implement an async handler on top of a blocking handler
    // // (use case: user provides us a blocking handler, but we are an async server,

    // impl<B, H, R> Handler<R> for Blocking<B, H>
    // where
    //     B: Blocker + Clone + Send,
    //     H: super::Handler<Blocking<B, R>>,
    //     R: Request,
    // {
    //     type HandleFuture<'a>
    //     where
    //         Self: 'a,
    //     = impl Future<Output = HandlerResult>;

    //     fn handle(&self, request: R) -> Self::HandleFuture<'_> {
    //         async move {
    //             self.1.handle(Blocking::new(self.0.clone(), request))
    //         }
    //     }
    // }

    pub trait Middleware<R>: Send
    where
        R: Request,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult> + Send
        where
            Self: 'a;

        fn handle<H>(&self, request: R, handler: &H) -> Self::HandleFuture<'_>
        where
            H: Handler<R>;

        fn compose<H>(self, handler: H) -> CompositeHandler<Self, H>
        where
            H: Handler<R>,
            Self: Sized,
        {
            CompositeHandler::new(self, handler)
        }
    }

    pub struct CompositeHandler<M, H> {
        middleware: M,
        handler: H,
    }

    impl<M, H> CompositeHandler<M, H> {
        pub fn new(middleware: M, handler: H) -> Self {
            Self {
                middleware,
                handler,
            }
        }
    }

    impl<M, H, R> Handler<R> for CompositeHandler<M, H>
    where
        M: Middleware<R>,
        H: Handler<R>,
        R: Request,
    {
        type HandleFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = HandlerResult> + Send;

        fn handle(&self, request: R) -> Self::HandleFuture<'_> {
            self.middleware.handle(request, &self.handler)
        }
    }
}
