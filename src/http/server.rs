use core::fmt::{self, Debug, Display, Write as _};

use crate::io::{Error, Read, Write};

pub use super::{Headers, Method, Query, Status};
pub use crate::io::Io;

pub trait Connection: Io {
    type Request;

    type Response;

    type Headers: Query + Headers;

    type Read: Read<Error = Self::Error>;

    type Write: Write<Error = Self::Error>;

    type RawConnectionError: Error;

    type RawConnection: Read<Error = Self::RawConnectionError>
        + Write<Error = Self::RawConnectionError>;

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

    fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> &'a mut Self::Write;

    fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error>;
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

    use crate::executor::asynch::{Blocker, RawBlocking, RawTrivialAsync};
    use crate::io::{asynch::Read, asynch::Write};

    pub use super::{HandlerError, HandlerResult, Headers, Method, Query, Status};
    pub use crate::io::{Error, Io};

    pub trait Connection: Io {
        type Request;

        type Response;

        type Headers: Query + Headers;

        type Read: Read<Error = Self::Error>;

        type Write: Write<Error = Self::Error>;

        type RawConnectionError: Error;

        type RawConnection: Read<Error = Self::RawConnectionError>
            + Write<Error = Self::RawConnectionError>;

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

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error>;
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

    pub struct BlockingConnection<B, C>
    where
        C: Connection,
    {
        blocker: B,
        connection: C,
        lended_read: RawBlocking<B, C::Read>,
        lended_write: RawBlocking<B, C::Write>,
        lended_raw: RawBlocking<B, C::RawConnection>,
    }

    impl<B, C> BlockingConnection<B, C>
    where
        C: Connection,
    {
        pub fn new(blocker: B, connection: C) -> Self {
            Self {
                blocker,
                connection,
                lended_read: unsafe { RawBlocking::new() },
                lended_write: unsafe { RawBlocking::new() },
                lended_raw: unsafe { RawBlocking::new() },
            }
        }
    }

    impl<B, C> Io for BlockingConnection<B, C>
    where
        C: Connection,
    {
        type Error = C::Error;
    }

    impl<B, C> super::Connection for BlockingConnection<B, C>
    where
        B: Blocker + Clone,
        C: Connection,
    {
        type Request = C::Request;

        type Response = C::Response;

        type Headers = C::Headers;

        type Read = RawBlocking<B, C::Read>;

        type Write = RawBlocking<B, C::Write>;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = RawBlocking<B, C::RawConnection>;

        fn split<'a>(
            &'a mut self,
            request: &'a mut Self::Request,
        ) -> (&'a Self::Headers, &'a mut Self::Read) {
            let (headers, read) = self.connection.split(request);

            self.lended_read.blocker = &self.blocker;
            self.lended_read.api = read;

            (headers, &mut self.lended_read)
        }

        fn headers<'a>(&'a self, request: &'a Self::Request) -> &'a Self::Headers {
            self.connection.headers(request)
        }

        fn reader<'a>(&'a mut self, request: &'a mut Self::Request) -> &'a mut Self::Read {
            let read = self.connection.reader(request);

            self.lended_read.blocker = &self.blocker;
            self.lended_read.api = read;

            &mut self.lended_read
        }

        fn into_response<'a>(
            &'a mut self,
            request: Self::Request,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<Self::Response, Self::Error> {
            let response = self.blocker.block_on(
                self.connection
                    .into_response(request, status, message, headers),
            )?;

            Ok(response)
        }

        fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> &'a mut Self::Write {
            let write = self.connection.writer(response);

            self.lended_write.blocker = &self.blocker;
            self.lended_write.api = write;

            &mut self.lended_write
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            let connection = self.connection.raw_connection()?;

            self.lended_raw.blocker = &self.blocker;
            self.lended_raw.api = connection;

            Ok(&mut self.lended_raw)
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

    pub struct TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        connection: C,
        lended_read: RawTrivialAsync<C::Read>,
        lended_write: RawTrivialAsync<C::Write>,
        lended_raw: RawTrivialAsync<C::RawConnection>,
    }

    impl<C> TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        pub fn new(connection: C) -> Self {
            Self {
                connection,
                lended_read: unsafe { RawTrivialAsync::new() },
                lended_write: unsafe { RawTrivialAsync::new() },
                lended_raw: unsafe { RawTrivialAsync::new() },
            }
        }

        pub fn api(&self) -> &C {
            &self.connection
        }

        pub fn api_mut(&mut self) -> &mut C {
            &mut self.connection
        }
    }

    impl<C> Io for TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        type Error = C::Error;
    }

    impl<C> Connection for TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        type Request = C::Request;

        type Response = C::Response;

        type Headers = C::Headers;

        type Read = RawTrivialAsync<C::Read>;

        type Write = RawTrivialAsync<C::Write>;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = RawTrivialAsync<C::RawConnection>;

        type IntoResponseFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<Self::Response, Self::Error>>;

        fn split<'a>(
            &'a mut self,
            request: &'a mut Self::Request,
        ) -> (&'a Self::Headers, &'a mut Self::Read) {
            let (headers, read) = self.connection.split(request);

            self.lended_read.api = read;

            (headers, &mut self.lended_read)
        }

        fn headers<'a>(&'a self, request: &'a Self::Request) -> &'a Self::Headers {
            self.connection.headers(request)
        }

        fn into_response<'a>(
            &'a mut self,
            request: Self::Request,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoResponseFuture<'a> {
            async move {
                Ok(self
                    .connection
                    .into_response(request, status, message, headers)?)
            }
        }

        fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> &'a mut Self::Write {
            let write = self.connection.writer(response);

            self.lended_write.api = write;

            &mut self.lended_write
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            let raw_connection = self.connection.raw_connection()?;

            self.lended_raw.api = raw_connection;

            Ok(&mut self.lended_raw)
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
}
