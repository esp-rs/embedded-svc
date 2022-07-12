use core::fmt::{self, Debug, Display, Write as _};

use crate::io::{Io, Read, Write};

pub use super::{Headers, Query, RequestId, SendHeaders, SendStatus};

struct PrivateData;

pub struct Completion(PrivateData);

impl Completion {
    pub unsafe fn internal_new() -> Self {
        Self(PrivateData)
    }
}

pub trait Request: RequestId + Query + Headers + Read {
    type Headers: RequestId + Query + Headers;
    type Body: RequestBody<Request = Self, Error = Self::Error>;

    type Response: Response<Error = Self::Error>;
    type ResponseHeaders: ResponseHeaders<Request = Self, Error = Self::Error>;

    fn split(self) -> (Self::Headers, Self::Body, Self::ResponseHeaders)
    where
        Self: Sized;

    fn into_response(self) -> Result<Self::Response, Self::Error>
    where
        Self: Sized;

    fn complete(self) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        self.into_response()?.complete()
    }
}

pub trait RequestBody: Read {
    type Request: Request<Body = Self, Error = Self::Error>;

    fn merge(
        self,
        request_headers: <<Self as RequestBody>::Request as Request>::Headers,
        response_headers: <<Self as RequestBody>::Request as Request>::ResponseHeaders,
    ) -> Self::Request
    where
        Self: Sized;
}

pub trait ResponseHeaders: SendStatus + SendHeaders + Io {
    type Request: Request<ResponseHeaders = Self, Error = Self::Error>;

    fn into_response(
        self,
        request_body: <<Self as ResponseHeaders>::Request as Request>::Body,
    ) -> Result<<<Self as ResponseHeaders>::Request as Request>::Response, Self::Error>
    where
        Self: Sized;
}

pub trait Response: SendStatus + SendHeaders + Io {
    type Write: ResponseWrite<Error = Self::Error>;

    fn into_writer(self) -> Result<Self::Write, Self::Error>
    where
        Self: Sized;

    fn submit(self, data: &[u8]) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        let mut write = self.into_writer()?;

        write.write_all(data)?;

        write.complete()
    }

    fn complete(self) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        self.into_writer()?.complete()
    }
}

pub trait ResponseWrite: Write {
    fn complete(self) -> Result<Completion, Self::Error>
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

pub type HandlerResult = Result<Completion, HandlerError>;

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

    use crate::io::{asynch::Read, asynch::Write, Io};
    use crate::unblocker::asynch::{Blocker, Blocking, TrivialAsync};

    pub use crate::http::{Headers, Query, RequestId, SendHeaders, SendStatus};

    use super::Completion;
    pub use super::{HandlerError, HandlerResult};

    pub trait Request: RequestId + Query + Headers + Read {
        type Headers: Query + RequestId + Headers;
        type Body: RequestBody<Request = Self, Error = Self::Error>;

        type Response: Response<Error = Self::Error>;
        type ResponseHeaders: ResponseHeaders<Request = Self, Error = Self::Error>;

        type IntoResponseFuture: Future<Output = Result<Self::Response, Self::Error>>;

        fn split(self) -> (Self::Headers, Self::Body, Self::ResponseHeaders)
        where
            Self: Sized;

        fn into_response(self) -> Self::IntoResponseFuture
        where
            Self: Sized;
    }

    pub trait RequestBody: Read {
        type Request: Request<Body = Self, Error = Self::Error>;

        fn merge(
            self,
            request_headers: <<Self as RequestBody>::Request as Request>::Headers,
            response_headers: <<Self as RequestBody>::Request as Request>::ResponseHeaders,
        ) -> Self::Request
        where
            Self: Sized;
    }

    pub trait ResponseHeaders: SendStatus + SendHeaders + Io {
        type Request: Request<ResponseHeaders = Self, Error = Self::Error>;

        type IntoResponseFuture: Future<
            Output = Result<<<Self as ResponseHeaders>::Request as Request>::Response, Self::Error>,
        >;

        fn into_response(
            self,
            request_body: <<Self as ResponseHeaders>::Request as Request>::Body,
        ) -> Self::IntoResponseFuture
        where
            Self: Sized;
    }

    pub trait Response: SendStatus + SendHeaders + Io {
        type Write: ResponseWrite<Error = Self::Error>;

        type IntoWriterFuture: Future<Output = Result<Self::Write, Self::Error>>;
        type SubmitFuture<'a>: Future<Output = Result<Completion, Self::Error>>;
        type CompleteFuture: Future<Output = Result<Completion, Self::Error>>;

        fn into_writer(self) -> Self::IntoWriterFuture
        where
            Self: Sized;

        fn submit<'a>(self, data: &'a [u8]) -> Self::SubmitFuture<'a>
        where
            Self: Sized;

        fn complete(self) -> Self::CompleteFuture
        where
            Self: Sized;
    }

    pub trait ResponseWrite: Write {
        type CompleteFuture: Future<Output = Result<Completion, Self::Error>>;

        fn complete(self) -> Self::CompleteFuture
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
        type Headers = R::Headers;
        type Body = Blocking<B, R::Body>;

        type Response = Blocking<B, R::Response>;
        type ResponseHeaders = Blocking<B, R::ResponseHeaders>;

        fn split(self) -> (Self::Headers, Self::Body, Self::ResponseHeaders) {
            let (headers, body, response_headers) = self.1.split();

            (
                headers,
                Blocking::new(self.0.clone(), body),
                Blocking::new(self.0, response_headers),
            )
        }

        fn into_response(self) -> Result<Self::Response, Self::Error>
        where
            Self: Sized,
        {
            let response = self.0.block_on(self.1.into_response())?;

            Ok(Blocking::new(self.0, response))
        }
    }

    impl<B, R> super::RequestBody for Blocking<B, R>
    where
        B: Blocker + Clone,
        R: RequestBody,
    {
        type Request = Blocking<B, R::Request>;

        fn merge(
            self,
            request_headers: <<Self as super::RequestBody>::Request as super::Request>::Headers,
            response_headers: <<Self as super::RequestBody>::Request as super::Request>::ResponseHeaders,
        ) -> Self::Request
        where
            Self: Sized,
        {
            Blocking::new(self.0, self.1.merge(request_headers, response_headers.1))
        }
    }

    impl<B, R> super::ResponseHeaders for Blocking<B, R>
    where
        B: Blocker + Clone,
        R: ResponseHeaders,
    {
        type Request = Blocking<B, R::Request>;

        fn into_response(
            self,
            request_body: Blocking<B, <<R as ResponseHeaders>::Request as Request>::Body>,
        ) -> Result<
            <<Self as super::ResponseHeaders>::Request as super::Request>::Response,
            Self::Error,
        >
        where
            Self: Sized,
        {
            let response = self.0.block_on(self.1.into_response(request_body.1))?;

            Ok(Blocking::new(self.0, response))
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
            let write = self.0.block_on(self.1.into_writer())?;

            Ok(Blocking::new(self.0, write))
        }
    }

    impl<B, R> super::ResponseWrite for Blocking<B, R>
    where
        B: Blocker,
        R: ResponseWrite,
    {
        fn complete(self) -> Result<Completion, Self::Error>
        where
            Self: Sized,
        {
            Ok(self.0.block_on(self.1.complete())?)
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
        type Headers = R::Headers;
        type Body = TrivialAsync<R::Body>;

        type Response = TrivialAsync<R::Response>;
        type ResponseHeaders = TrivialAsync<R::ResponseHeaders>;

        type IntoResponseFuture = impl Future<Output = Result<Self::Response, Self::Error>>;

        fn split(self) -> (Self::Headers, Self::Body, Self::ResponseHeaders) {
            let (headers, body, response_headers) = self.1.split();

            (
                headers,
                TrivialAsync::new_async(body),
                TrivialAsync::new_async(response_headers),
            )
        }

        fn into_response(self) -> Self::IntoResponseFuture
        where
            Self: Sized,
        {
            async move { Ok(TrivialAsync::new_async(self.1.into_response()?)) }
        }
    }

    impl<R> RequestBody for TrivialAsync<R>
    where
        R: super::RequestBody,
    {
        type Request = TrivialAsync<R::Request>;

        fn merge(
            self,
            request_headers: <<Self as RequestBody>::Request as Request>::Headers,
            response_headers: <<Self as RequestBody>::Request as Request>::ResponseHeaders,
        ) -> Self::Request
        where
            Self: Sized,
        {
            TrivialAsync::new_async(self.1.merge(request_headers, response_headers.1))
        }
    }

    impl<R> ResponseHeaders for TrivialAsync<R>
    where
        R: super::ResponseHeaders,
    {
        type Request = TrivialAsync<R::Request>;

        type IntoResponseFuture = impl Future<
            Output = Result<<<Self as ResponseHeaders>::Request as Request>::Response, Self::Error>,
        >;

        fn into_response(
            self,
            request_body: <<Self as ResponseHeaders>::Request as Request>::Body,
        ) -> Self::IntoResponseFuture
        where
            Self: Sized,
        {
            async move {
                Ok(TrivialAsync::new_async(
                    self.1.into_response(request_body.1)?,
                ))
            }
        }
    }

    impl<R> ResponseWrite for TrivialAsync<R>
    where
        R: super::ResponseWrite,
    {
        type CompleteFuture = impl Future<Output = Result<Completion, Self::Error>>;

        fn complete(self) -> Self::CompleteFuture
        where
            Self: Sized,
        {
            async move { self.1.complete() }
        }
    }

    impl<R> Response for TrivialAsync<R>
    where
        R: super::Response,
    {
        type Write = TrivialAsync<R::Write>;

        type IntoWriterFuture = impl Future<Output = Result<Self::Write, Self::Error>>;
        type SubmitFuture<'a> = impl Future<Output = Result<Completion, Self::Error>>;
        type CompleteFuture = impl Future<Output = Result<Completion, Self::Error>>;

        fn into_writer(self) -> Self::IntoWriterFuture
        where
            Self: Sized,
        {
            async move { self.1.into_writer().map(TrivialAsync::new_async) }
        }

        fn submit<'a>(self, data: &'a [u8]) -> Self::SubmitFuture<'a>
        where
            Self: Sized,
        {
            async move { self.1.submit(data) }
        }

        fn complete(self) -> Self::CompleteFuture
        where
            Self: Sized,
        {
            async move { self.1.complete() }
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
