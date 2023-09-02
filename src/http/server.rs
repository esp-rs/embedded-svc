use core::fmt::{self, Debug, Display, Write as _};

use crate::io::{Error, Read, Write};

pub use super::{Headers, Method, Query, Status};
pub use crate::io::ErrorType;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Request<C>(C);

impl<C> Request<C>
where
    C: Connection,
{
    pub fn wrap(connection: C) -> Request<C> {
        if connection.is_response_initiated() {
            panic!("connection is not in request phase");
        }

        Request(connection)
    }

    pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
        self.0.split()
    }

    pub fn into_response<'b>(
        mut self,
        status: u16,
        message: Option<&'b str>,
        headers: &'b [(&'b str, &'b str)],
    ) -> Result<Response<C>, C::Error> {
        self.0.initiate_response(status, message, headers)?;

        Ok(Response(self.0))
    }

    pub fn into_status_response(self, status: u16) -> Result<Response<C>, C::Error> {
        self.into_response(status, None, &[])
    }

    pub fn into_ok_response(self) -> Result<Response<C>, C::Error> {
        self.into_response(200, Some("OK"), &[])
    }

    pub fn connection(&mut self) -> &mut C {
        &mut self.0
    }

    pub fn release(self) -> C {
        self.0
    }

    pub fn uri(&self) -> &'_ str {
        self.0.uri()
    }

    pub fn method(&self) -> Method {
        self.0.method()
    }

    pub fn header(&self, name: &str) -> Option<&'_ str> {
        self.0.header(name)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, C::Error> {
        self.0.read(buf)
    }
}

impl<C> ErrorType for Request<C>
where
    C: ErrorType,
{
    type Error = C::Error;
}

impl<C> Read for Request<C>
where
    C: Connection,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Request::read(self, buf)
    }
}

impl<C> Headers for Request<C>
where
    C: Connection,
{
    fn header(&self, name: &str) -> Option<&'_ str> {
        Request::header(self, name)
    }
}

impl<C> Query for Request<C>
where
    C: Connection,
{
    fn uri(&self) -> &'_ str {
        Request::uri(self)
    }

    fn method(&self) -> Method {
        Request::method(self)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Response<C>(C);

impl<C> Response<C>
where
    C: Connection,
{
    pub fn wrap(connection: C) -> Response<C> {
        if !connection.is_response_initiated() {
            panic!("connection is not in response phase");
        }

        Response(connection)
    }

    pub fn connection(&mut self) -> &mut C {
        &mut self.0
    }

    pub fn release(self) -> C {
        self.0
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, C::Error> {
        self.0.write(buf)
    }

    pub fn flush(&mut self) -> Result<(), C::Error> {
        self.0.flush()
    }
}

impl<C> ErrorType for Response<C>
where
    C: ErrorType,
{
    type Error = C::Error;
}

impl<C> Write for Response<C>
where
    C: Connection,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Response::write(self, buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Response::flush(self)
    }
}

pub trait Connection: Query + Headers + Read + Write {
    type Headers: Query + Headers;

    type Read: Read<Error = Self::Error>;

    type RawConnectionError: Error;

    type RawConnection: Read<Error = Self::RawConnectionError>
        + Write<Error = Self::RawConnectionError>;

    fn split(&mut self) -> (&Self::Headers, &mut Self::Read);

    fn initiate_response<'a>(
        &'a mut self,
        status: u16,
        message: Option<&'a str>,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<(), Self::Error>;

    fn is_response_initiated(&self) -> bool;

    fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error>;
}

impl<C> Connection for &mut C
where
    C: Connection,
{
    type Headers = C::Headers;

    type Read = C::Read;

    type RawConnectionError = C::RawConnectionError;

    type RawConnection = C::RawConnection;

    fn split(&mut self) -> (&Self::Headers, &mut Self::Read) {
        (*self).split()
    }

    fn initiate_response<'a>(
        &'a mut self,
        status: u16,
        message: Option<&'a str>,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<(), Self::Error> {
        (*self).initiate_response(status, message, headers)
    }

    fn is_response_initiated(&self) -> bool {
        (**self).is_response_initiated()
    }

    fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
        (*self).raw_connection()
    }
}

pub struct HandlerError(heapless::String<64>);

impl HandlerError {
    pub fn new(message: &str) -> Self {
        Self(message.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }

    pub fn release(self) -> heapless::String<64> {
        self.0
    }
}

impl<E> From<E> for HandlerError
where
    E: Debug,
{
    fn from(e: E) -> Self {
        let mut string: heapless::String<64> = "".into();

        if write!(&mut string, "{e:?}").is_err() {
            string = "(Error string too big)".into();
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
    fn handle(&self, connection: &mut C) -> HandlerResult;
}

impl<C, H> Handler<C> for &H
where
    C: Connection,
    H: Handler<C> + Send + Sync,
{
    fn handle(&self, connection: &mut C) -> HandlerResult {
        (*self).handle(connection)
    }
}

pub struct FnHandler<F>(F);

impl<F> FnHandler<F> {
    pub const fn new<C>(f: F) -> Self
    where
        C: Connection,
        F: Fn(Request<&mut C>) -> HandlerResult + Send,
    {
        Self(f)
    }
}

impl<C, F> Handler<C> for FnHandler<F>
where
    C: Connection,
    F: Fn(Request<&mut C>) -> HandlerResult + Send,
{
    fn handle(&self, connection: &mut C) -> HandlerResult {
        self.0(Request::wrap(connection))
    }
}

pub trait Middleware<C>: Send
where
    C: Connection,
{
    fn handle<'a, H>(&'a self, connection: &'a mut C, handler: &'a H) -> HandlerResult
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
    fn handle(&self, connection: &mut C) -> HandlerResult {
        self.middleware.handle(connection, &self.handler)
    }
}

#[cfg(feature = "nightly")]
pub mod asynch {
    use crate::io::{asynch::Read, asynch::Write};

    pub use super::{HandlerError, HandlerResult, Headers, Method, Query, Status};
    pub use crate::io::{Error, ErrorType};

    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Request<C>(C);

    impl<C> Request<C>
    where
        C: Connection,
    {
        pub fn wrap(connection: C) -> Request<C> {
            if connection.is_response_initiated() {
                panic!("connection is not in request phase");
            }

            Request(connection)
        }

        pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
            self.0.split()
        }

        pub async fn into_response<'b>(
            mut self,
            status: u16,
            message: Option<&'b str>,
            headers: &'b [(&'b str, &'b str)],
        ) -> Result<Response<C>, C::Error> {
            self.0.initiate_response(status, message, headers).await?;

            Ok(Response(self.0))
        }

        pub async fn into_status_response(self, status: u16) -> Result<Response<C>, C::Error> {
            self.into_response(status, None, &[]).await
        }

        pub async fn into_ok_response(self) -> Result<Response<C>, C::Error> {
            self.into_response(200, Some("OK"), &[]).await
        }

        pub fn connection(&mut self) -> &mut C {
            &mut self.0
        }

        pub fn release(self) -> C {
            self.0
        }

        pub fn uri(&self) -> &'_ str {
            self.0.uri()
        }

        pub fn method(&self) -> Method {
            self.0.method()
        }

        pub fn header(&self, name: &str) -> Option<&'_ str> {
            self.0.header(name)
        }

        pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, C::Error> {
            self.0.read(buf).await
        }
    }

    impl<C> ErrorType for Request<C>
    where
        C: ErrorType,
    {
        type Error = C::Error;
    }

    impl<C> Read for Request<C>
    where
        C: Connection,
    {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            Request::read(self, buf).await
        }
    }

    impl<C> Query for Request<C>
    where
        C: Connection,
    {
        fn uri(&self) -> &'_ str {
            Request::uri(self)
        }

        fn method(&self) -> Method {
            Request::method(self)
        }
    }

    impl<C> Headers for Request<C>
    where
        C: Connection,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            Request::header(self, name)
        }
    }

    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Response<C>(C);

    impl<C> Response<C>
    where
        C: Connection,
    {
        pub fn wrap(connection: C) -> Response<C> {
            if !connection.is_response_initiated() {
                panic!("connection is not in response phase");
            }

            Response(connection)
        }

        pub fn connection(&mut self) -> &mut C {
            &mut self.0
        }

        pub fn release(self) -> C {
            self.0
        }

        pub async fn write(&mut self, buf: &[u8]) -> Result<usize, C::Error> {
            self.0.write(buf).await
        }

        pub async fn flush(&mut self) -> Result<(), C::Error> {
            self.0.flush().await
        }
    }

    impl<C> ErrorType for Response<C>
    where
        C: ErrorType,
    {
        type Error = C::Error;
    }

    impl<C> Write for Response<C>
    where
        C: Connection,
    {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            Response::write(self, buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            Response::flush(self).await
        }
    }

    pub trait Connection: Query + Headers + Read + Write {
        type Headers: Query + Headers;

        type Read: Read<Error = Self::Error>;

        type RawConnectionError: Error;

        type RawConnection: Read<Error = Self::RawConnectionError>
            + Write<Error = Self::RawConnectionError>;

        fn split(&mut self) -> (&Self::Headers, &mut Self::Read);

        async fn initiate_response(
            &mut self,
            status: u16,
            message: Option<&str>,
            headers: &[(&str, &str)],
        ) -> Result<(), Self::Error>;

        fn is_response_initiated(&self) -> bool;

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error>;
    }

    impl<C> Connection for &mut C
    where
        C: Connection,
    {
        type Headers = C::Headers;

        type Read = C::Read;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = C::RawConnection;

        fn split(&mut self) -> (&Self::Headers, &mut Self::Read) {
            (*self).split()
        }

        async fn initiate_response(
            &mut self,
            status: u16,
            message: Option<&str>,
            headers: &[(&str, &str)],
        ) -> Result<(), Self::Error> {
            (*self).initiate_response(status, message, headers).await
        }

        fn is_response_initiated(&self) -> bool {
            (**self).is_response_initiated()
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            (*self).raw_connection()
        }
    }

    pub trait Handler<C>: Send
    where
        C: Connection,
    {
        async fn handle(&self, connection: &mut C) -> HandlerResult;
    }

    impl<H, C> Handler<C> for &H
    where
        C: Connection,
        H: Handler<C> + Send + Sync,
    {
        async fn handle(&self, connection: &mut C) -> HandlerResult {
            (*self).handle(connection).await
        }
    }

    pub trait Middleware<C>: Send
    where
        C: Connection,
    {
        async fn handle<H>(&self, connection: &mut C, handler: &H) -> HandlerResult
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
        async fn handle(&self, connection: &mut C) -> HandlerResult {
            self.middleware.handle(connection, &self.handler).await
        }
    }
}
