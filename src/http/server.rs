use core::fmt::{self, Debug, Display, Write as _};

use crate::io::{Read, Write};

pub use super::{Headers, Method, Query, Status};
pub use crate::io::Io;

pub trait Connection: Io {
    type Request;

    type Response;

    type Headers: Query + Headers;

    type Read: Read<Error = Self::Error>;

    type Write: Write<Error = Self::Error>;

    fn split<'a>(
        &'a mut self,
        request: &'a mut Self::Request,
    ) -> (&'a Self::Headers, &'a mut Self::Read);

    fn headers<'a>(&'a mut self, request: &'a mut Self::Request) -> &'a Self::Headers {
        let (header, _) = self.split(request);

        header
    }

    fn reader<'a>(&'a mut self, request: &'a mut Self::Request) -> &'a mut Self::Read {
        let (_, read) = self.split(request);

        read
    }

    fn into_response<'a>(
        &'a mut self,
        request: Self::Request,
        status: u16,
        message: Option<&'a str>,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Self::Response, Self::Error>;

    fn into_status_response(
        &mut self,
        request: Self::Request,
        status: u16,
    ) -> Result<Self::Response, Self::Error> {
        self.into_response(request, status, None, &[])
    }

    fn into_ok_response(&mut self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        self.into_response(request, 200, Some("OK"), &[])
    }

    fn writer<'a>(&'a mut self, response: &'a Self::Response) -> &'a mut Self::Write;
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

pub trait Handler<C>: Send
where
    C: Connection,
{
    fn handle<'a>(&'a self, connection: &'a mut C, request: C::Request) -> HandlerResult;
}

impl<C, H> Handler<C> for &H
where
    C: Connection,
    H: Handler<C> + Send + Sync,
{
    fn handle<'a>(&'a self, connection: &'a mut C, request: C::Request) -> HandlerResult {
        (*self).handle(connection, request)
    }
}

pub struct FnHandler<F>(F);

impl<F> FnHandler<F> {
    pub const fn new<C>(f: F) -> Self
    where
        C: Connection,
        F: Fn(&mut C, C::Request) -> HandlerResult,
    {
        Self(f)
    }
}

impl<C, F> Handler<C> for FnHandler<F>
where
    C: Connection,
    F: Fn(&mut C, C::Request) -> HandlerResult + Send,
{
    fn handle<'a>(&'a self, connection: &'a mut C, request: C::Request) -> HandlerResult {
        self.0(connection, request)
    }
}

pub trait Middleware<C>: Send
where
    C: Connection,
{
    fn handle<'a, H>(
        &'a self,
        connection: &'a mut C,
        request: C::Request,
        handler: &'a H,
    ) -> HandlerResult
    where
        H: Handler<C>;

    fn compose<H>(self, handler: H) -> CompositeHandler<Self, H>
    where
        H: Handler<C>,
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

impl<M, H, C> Handler<C> for CompositeHandler<M, H>
where
    M: Middleware<C>,
    H: Handler<C>,
    C: Connection,
{
    fn handle<'a>(&'a self, connection: &'a mut C, request: C::Request) -> HandlerResult {
        self.middleware.handle(connection, request, &self.handler)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::io::{asynch::Read, asynch::Write};
    //use crate::unblocker::asynch::{Blocker, Blocking, TrivialAsync};

    pub use super::{HandlerError, HandlerResult, Headers, Method, Query, Status};
    pub use crate::io::Io;

    pub trait Connection: Io {
        type Request;

        type Response;

        type Headers: Query + Headers;

        type Read: Read<Error = Self::Error>;

        type Write: Write<Error = Self::Error>;

        type IntoResponseFuture<'a>: Future<Output = Result<Self::Response, Self::Error>>
        where
            Self: 'a;

        fn split<'a>(
            &'a mut self,
            request: &'a mut Self::Request,
        ) -> (&'a Self::Headers, &'a mut Self::Read);

        fn headers<'a>(&'a self, request: &'a Self::Request) -> &'a Self::Headers;

        fn reader<'a>(&'a mut self, request: &'a mut Self::Request) -> &'a mut Self::Read {
            let (_, read) = self.split(request);

            read
        }

        fn into_response<'a>(
            &'a mut self,
            request: Self::Request,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoResponseFuture<'a>;

        fn into_status_response<'a>(
            &'a mut self,
            request: Self::Request,
            status: u16,
        ) -> Self::IntoResponseFuture<'a> {
            self.into_response(request, status, None, &[])
        }

        fn into_ok_response<'a>(
            &'a mut self,
            request: Self::Request,
        ) -> Self::IntoResponseFuture<'a> {
            self.into_response(request, 200, None, &[])
        }

        fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> &'a mut Self::Write;
    }

    pub trait Handler<C>: Send
    where
        C: Connection,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult>
        where
            Self: 'a,
            C: 'a;

        fn handle<'a>(
            &'a self,
            connection: &'a mut C,
            request: C::Request,
        ) -> Self::HandleFuture<'a>;
    }

    impl<H, C> Handler<C> for &H
    where
        C: Connection,
        H: Handler<C> + Send + Sync,
    {
        type HandleFuture<'a>
        where
            Self: 'a,
            C: 'a,
        = H::HandleFuture<'a>;

        fn handle<'a>(
            &'a self,
            connection: &'a mut C,
            request: C::Request,
        ) -> Self::HandleFuture<'a> {
            (*self).handle(connection, request)
        }
    }

    // impl<B, C> super::Connection for Blocking<B, C>
    // where
    //     B: Blocker + Clone,
    //     C: Connection,
    // {
    //     type Request = C::Request;

    //     type Response = C::Response;

    //     type Headers<'a>
    //     where
    //         Self: 'a,
    //     = C::Headers<'a>;

    //     type Read<'a>
    //     where
    //         Self: 'a,
    //     = Blocking<B, C::Read<'a>>;

    //     type Write<'a>
    //     where
    //         Self: 'a,
    //     = Blocking<B, C::Write<'a>>;

    //     fn reader<'a>(&'a mut self, request: &'a mut Self::Request) -> (Self::Headers<'a>, Self::Read<'a>) {
    //         let (headers, read) = self.1.reader(request);

    //         (headers, Blocking::new(self.0.clone(), read))
    //     }

    //     fn into_response<'a>(
    //         &'a mut self,
    //         request: Self::Request,
    //         status: u16,
    //         message: Option<&'a str>,
    //         headers: &'a [(&'a str, &'a str)],
    //     ) -> Result<Self::Response, Self::Error> {
    //         let response = self
    //             .0
    //             .block_on(self.1.into_response(request, status, message, headers))?;

    //         Ok(response)
    //     }

    //     fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> Self::Write<'a> {
    //         Blocking::new(self.0.clone(), self.1.writer(response))
    //     }
    // }

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

    // impl<C> Connection for TrivialAsync<C>
    // where
    //     C: super::Connection,
    // {
    //     type Request = C::Request;

    //     type Response = C::Response;

    //     type Headers<'a>
    //     where
    //         Self: 'a,
    //     = C::Headers<'a>;

    //     type Read<'a>
    //     where
    //         Self: 'a,
    //     = TrivialAsync<C::Read<'a>>;

    //     type Write<'a>
    //     where
    //         Self: 'a,
    //     = TrivialAsync<C::Write<'a>>;

    //     type IntoResponseFuture<'a> =
    //         impl Future<Output = Result<Self::Response, Self::Error>>;

    //     fn reader<'a>(&'a mut self, request: &'a mut Self::Request) -> (Self::Headers<'a>, Self::Read<'a>) {
    //         let (headers, reader) = self.1.reader(request);

    //         (headers, TrivialAsync::new_async(reader))
    //     }

    //     fn into_response<'a>(
    //         &'a mut self,
    //         request: Self::Request,
    //         status: u16,
    //         message: Option<&'a str>,
    //         headers: &'a [(&'a str, &'a str)],
    //     ) -> Self::IntoResponseFuture<'a> {
    //         async move {
    //             Ok(TrivialAsync::new_async(
    //                 self.1.into_response(request, status, message, headers)?,
    //             ))
    //         }
    //     }

    //     fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> Self::Write<'a> {
    //         TrivialAsync::new_async(self.1.writer(response))
    //     }
    // }

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

    pub trait Middleware<C>: Send
    where
        C: Connection,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult> + Send
        where
            Self: 'a,
            C: 'a;

        fn handle<'a, H>(
            &'a self,
            connection: &'a mut C,
            request: C::Request,
            handler: &'a H,
        ) -> Self::HandleFuture<'a>
        where
            H: Handler<C>;

        fn compose<H>(self, handler: H) -> CompositeHandler<Self, H>
        where
            H: Handler<C>,
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

    impl<M, H, C> Handler<C> for CompositeHandler<M, H>
    where
        M: Middleware<C>,
        H: Handler<C>,
        C: Connection,
    {
        type HandleFuture<'a>
        where
            Self: 'a,
            C: 'a,
        = impl Future<Output = HandlerResult> + Send;

        fn handle<'a>(
            &'a self,
            connection: &'a mut C,
            request: C::Request,
        ) -> Self::HandleFuture<'a> {
            self.middleware.handle(connection, request, &self.handler)
        }
    }
}
