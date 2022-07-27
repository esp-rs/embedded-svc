use core::fmt::{self, Debug, Display, Write as _};

use crate::io::{Error, Read, Write};

pub use super::{Headers, Method, Query, Status};
pub use crate::io::Io;

#[derive(Debug)]
pub struct Request<C>(C);

impl<C> Request<C>
where
    C: Connection,
{
    pub fn wrap(mut connection: C) -> Result<Request<C>, C::Error> {
        connection.request()?;

        Ok(Request(connection))
    }

    pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
        self.0.request().unwrap()
    }

    pub fn into_response<'b>(
        mut self,
        status: u16,
        message: Option<&'b str>,
        headers: &'b [(&'b str, &'b str)],
    ) -> Result<Response<C>, C::Error> {
        self.0.into_response(status, message, headers)?;

        Ok(Response(self.0))
    }

    pub fn into_status_response<'b>(self, status: u16) -> Result<Response<C>, C::Error> {
        self.into_response(status, None, &[])
    }

    pub fn into_ok_response<'b>(self) -> Result<Response<C>, C::Error> {
        self.into_response(200, Some("OK"), &[])
    }

    pub fn release(self) -> C {
        self.0
    }
}

impl<C> Io for Request<C>
where
    C: Io,
{
    type Error = C::Error;
}

impl<C> Read for Request<C>
where
    C: Connection,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.request().unwrap().1.read(buf)
    }
}

impl<C> Headers for Request<C>
where
    C: Connection,
{
    fn header(&self, name: &str) -> Option<&'_ str> {
        self.0.headers().unwrap().header(name)
    }
}

#[derive(Debug)]
pub struct Response<C>(C);

impl<C> Response<C>
where
    C: Connection,
{
    pub fn wrap(mut connection: C) -> Result<Response<C>, C::Error> {
        connection.response()?;

        Ok(Response(connection))
    }

    pub fn release(self) -> C {
        self.0
    }
}

impl<C> Io for Response<C>
where
    C: Io,
{
    type Error = C::Error;
}

impl<C> Write for Response<C>
where
    C: Connection,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.response().unwrap().write(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.response().unwrap().flush()
    }
}

pub trait Connection: Io {
    type Headers: Query + Headers;

    type Read: Read<Error = Self::Error>;

    type Write: Write<Error = Self::Error>;

    type RawConnectionError: Error;

    type RawConnection: Read<Error = Self::RawConnectionError>
        + Write<Error = Self::RawConnectionError>;

    fn headers<'a>(&'a self) -> Result<&'a Self::Headers, Self::Error>;
    fn request<'a>(&'a mut self) -> Result<(&'a Self::Headers, &'a mut Self::Read), Self::Error>;

    fn into_response<'a>(
        &'a mut self,
        status: u16,
        message: Option<&'a str>,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<(), Self::Error>;

    fn response<'a>(&'a mut self) -> Result<&'a mut Self::Write, Self::Error>;

    fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error>;
}

impl<C> Connection for &mut C
where
    C: Connection,
{
    type Headers = C::Headers;

    type Read = C::Read;

    type Write = C::Write;

    type RawConnectionError = C::RawConnectionError;

    type RawConnection = C::RawConnection;

    fn headers<'a>(&'a self) -> Result<&'a Self::Headers, Self::Error> {
        (**self).headers()
    }

    fn request<'a>(&'a mut self) -> Result<(&'a Self::Headers, &'a mut Self::Read), Self::Error> {
        (*self).request()
    }

    fn into_response<'a>(
        &'a mut self,
        status: u16,
        message: Option<&'a str>,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<(), Self::Error> {
        (*self).into_response(status, message, headers)
    }

    fn response<'a>(&'a mut self) -> Result<&'a mut Self::Write, Self::Error> {
        (*self).response()
    }

    fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
        (*self).raw_connection()
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

pub trait Handler<C>: Send
where
    C: Connection,
{
    fn handle(&self, connection: C) -> HandlerResult;
}

impl<C, H> Handler<C> for &H
where
    C: Connection,
    H: Handler<C> + Send + Sync,
{
    fn handle(&self, connection: C) -> HandlerResult {
        (*self).handle(connection)
    }
}

pub struct FnHandler<F>(F);

impl<F> FnHandler<F> {
    pub const fn new<C>(f: F) -> Self
    where
        C: Connection,
        F: Fn(C) -> HandlerResult + Send,
    {
        Self(f)
    }
}

impl<C, F> Handler<C> for FnHandler<F>
where
    C: Connection,
    F: Fn(C) -> HandlerResult + Send,
{
    fn handle(&self, connection: C) -> HandlerResult {
        self.0(connection)
    }
}

pub struct FnRequestHandler<F>(F);

impl<F> FnRequestHandler<F> {
    pub const fn new<C>(f: F) -> Self
    where
        C: Connection,
        F: for<'a> Fn(Request<C>) -> HandlerResult + Send,
    {
        Self(f)
    }
}

impl<C, F> Handler<C> for FnRequestHandler<F>
where
    C: Connection,
    F: for<'a> Fn(Request<C>) -> HandlerResult + Send,
{
    fn handle<'a>(&'a self, connection: C) -> HandlerResult {
        self.0(Request::wrap(connection)?)
    }
}

pub trait Middleware<C>: Send
where
    C: Connection,
{
    fn handle<'a, H>(&'a self, connection: C, handler: &'a H) -> HandlerResult
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
    fn handle<'a>(&'a self, connection: C) -> HandlerResult {
        self.middleware.handle(connection, &self.handler)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::executor::asynch::{Blocker, RawBlocking, RawTrivialAsync};
    use crate::io::{asynch::Read, asynch::Write};

    pub use super::{HandlerError, HandlerResult, Headers, Method, Query, Status};
    pub use crate::io::{Error, Io};

    #[derive(Debug)]
    pub struct Request<C>(C);

    impl<C> Request<C>
    where
        C: Connection,
    {
        pub fn wrap(mut connection: C) -> Result<Request<C>, C::Error> {
            connection.request()?;

            Ok(Request(connection))
        }

        pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
            self.0.request().unwrap()
        }

        pub async fn into_response<'b>(
            mut self,
            status: u16,
            message: Option<&'b str>,
            headers: &'b [(&'b str, &'b str)],
        ) -> Result<Response<C>, C::Error> {
            self.0.into_response(status, message, headers).await?;

            Ok(Response(self.0))
        }

        pub async fn into_status_response<'b>(self, status: u16) -> Result<Response<C>, C::Error> {
            self.into_response(status, None, &[]).await
        }

        pub async fn into_ok_response<'b>(self) -> Result<Response<C>, C::Error> {
            self.into_response(200, Some("OK"), &[]).await
        }

        pub fn release(self) -> C {
            self.0
        }
    }

    impl<C> Io for Request<C>
    where
        C: Io,
    {
        type Error = C::Error;
    }

    impl<C> Read for Request<C>
    where
        C: Connection,
    {
        type ReadFuture<'b>
        where
            Self: 'b,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn read<'b>(&'b mut self, buf: &'b mut [u8]) -> Self::ReadFuture<'b> {
            async move { self.0.request().unwrap().1.read(buf).await }
        }
    }

    impl<C> Headers for Request<C>
    where
        C: Connection,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.0.headers().unwrap().header(name)
        }
    }

    #[derive(Debug)]
    pub struct Response<C>(C);

    impl<C> Response<C>
    where
        C: Connection,
    {
        pub fn wrap(mut connection: C) -> Result<Response<C>, C::Error> {
            connection.response()?;

            Ok(Response(connection))
        }

        pub fn release(self) -> C {
            self.0
        }
    }

    impl<C> Io for Response<C>
    where
        C: Io,
    {
        type Error = C::Error;
    }

    impl<C> Write for Response<C>
    where
        C: Connection,
    {
        type WriteFuture<'b>
        where
            Self: 'b,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn write<'b>(&'b mut self, buf: &'b [u8]) -> Self::WriteFuture<'b> {
            async move { self.0.response().unwrap().write(buf).await }
        }

        type FlushFuture<'b>
        where
            Self: 'b,
        = impl Future<Output = Result<(), Self::Error>>;

        fn flush<'b>(&'b mut self) -> Self::FlushFuture<'b> {
            async move { self.0.response().unwrap().flush().await }
        }
    }

    pub trait Connection: Io {
        type Headers: Query + Headers;

        type Read: Read<Error = Self::Error>;

        type Write: Write<Error = Self::Error>;

        type RawConnectionError: Error;

        type RawConnection: Read<Error = Self::RawConnectionError>
            + Write<Error = Self::RawConnectionError>;

        type IntoResponseFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        fn headers<'a>(&'a self) -> Result<&'a Self::Headers, Self::Error>;
        fn request<'a>(
            &'a mut self,
        ) -> Result<(&'a Self::Headers, &'a mut Self::Read), Self::Error>;

        fn into_response<'a>(
            &'a mut self,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoResponseFuture<'a>;

        fn response<'a>(&'a mut self) -> Result<&'a mut Self::Write, Self::Error>;

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error>;
    }

    impl<C> Connection for &mut C
    where
        C: Connection,
    {
        type Headers = C::Headers;

        type Read = C::Read;

        type Write = C::Write;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = C::RawConnection;

        type IntoResponseFuture<'a>
        where
            Self: 'a,
        = C::IntoResponseFuture<'a>;

        fn headers<'a>(&'a self) -> Result<&'a Self::Headers, Self::Error> {
            (**self).headers()
        }

        fn request<'a>(
            &'a mut self,
        ) -> Result<(&'a Self::Headers, &'a mut Self::Read), Self::Error> {
            (*self).request()
        }

        fn into_response<'a>(
            &'a mut self,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoResponseFuture<'a> {
            (*self).into_response(status, message, headers)
        }

        fn response<'a>(&'a mut self) -> Result<&'a mut Self::Write, Self::Error> {
            (*self).response()
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            (*self).raw_connection()
        }
    }

    pub trait Handler<C>: Send
    where
        C: Connection,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult>
        where
            Self: 'a,
            C: 'a;

        fn handle<'a>(&'a self, connection: C) -> Self::HandleFuture<'a>;
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

        fn handle<'a>(&'a self, connection: C) -> Self::HandleFuture<'a> {
            (*self).handle(connection)
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

        fn handle<'a, H>(&'a self, connection: C, handler: &'a H) -> Self::HandleFuture<'a>
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

        fn handle<'a>(&'a self, connection: C) -> Self::HandleFuture<'a> {
            self.middleware.handle(connection, &self.handler)
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
        B: Blocker,
        C: Connection,
    {
        type Headers = C::Headers;

        type Read = RawBlocking<B, C::Read>;

        type Write = RawBlocking<B, C::Write>;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = RawBlocking<B, C::RawConnection>;

        fn headers<'a>(&'a self) -> Result<&'a Self::Headers, Self::Error> {
            self.connection.headers()
        }

        fn request<'a>(
            &'a mut self,
        ) -> Result<(&'a Self::Headers, &'a mut Self::Read), Self::Error> {
            let (headers, read) = self.connection.request()?;

            self.lended_read.blocker = &self.blocker;
            self.lended_read.api = read;

            Ok((headers, &mut self.lended_read))
        }

        fn into_response<'a>(
            &'a mut self,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<(), Self::Error> {
            self.blocker
                .block_on(self.connection.into_response(status, message, headers))?;

            Ok(())
        }

        fn response<'a>(&'a mut self) -> Result<&'a mut Self::Write, Self::Error> {
            let write = self.connection.response()?;

            self.lended_write.blocker = &self.blocker;
            self.lended_write.api = write;

            Ok(&mut self.lended_write)
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
        type Headers = C::Headers;

        type Read = RawTrivialAsync<C::Read>;

        type Write = RawTrivialAsync<C::Write>;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = RawTrivialAsync<C::RawConnection>;

        type IntoResponseFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>>;

        fn headers<'a>(&'a self) -> Result<&'a Self::Headers, Self::Error> {
            self.connection.headers()
        }

        fn request<'a>(
            &'a mut self,
        ) -> Result<(&'a Self::Headers, &'a mut Self::Read), Self::Error> {
            let (headers, read) = self.connection.request()?;

            self.lended_read.api = read;

            Ok((headers, &mut self.lended_read))
        }

        fn response<'a>(&'a mut self) -> Result<&'a mut Self::Write, Self::Error> {
            let write = self.connection.response()?;
            self.lended_write.api = write;

            Ok(&mut self.lended_write)
        }

        fn into_response<'a>(
            &'a mut self,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoResponseFuture<'a> {
            async move { self.connection.into_response(status, message, headers) }
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
