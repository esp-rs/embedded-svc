use crate::io::{Error, Io, Read, Write};

pub use super::{Headers, Method, Status};

#[derive(Debug)]
pub struct Client<C>(C);

impl<C> Client<C>
where
    C: Connection,
{
    pub const fn wrap(connection: C) -> Self {
        Self(connection)
    }

    pub fn release(self) -> C {
        self.0
    }

    pub fn get<'a>(&'a mut self, uri: &'a str) -> Result<Request<&'a mut C>, C::Error> {
        self.request(Method::Get, uri, &[])
    }

    pub fn post<'a>(
        &'a mut self,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Request<&'a mut C>, C::Error> {
        self.request(Method::Post, uri, headers)
    }

    pub fn put<'a>(
        &'a mut self,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Request<&'a mut C>, C::Error> {
        self.request(Method::Put, uri, headers)
    }

    pub fn delete<'a>(&'a mut self, uri: &'a str) -> Result<Request<&'a mut C>, C::Error> {
        self.request(Method::Delete, uri, &[])
    }

    pub fn request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Request<&'a mut C>, C::Error> {
        self.0.initiate_request(method, uri, headers)?;

        Request::wrap(&mut self.0)
    }

    pub fn raw_connection(&mut self) -> Result<&mut C::RawConnection, C::Error> {
        self.0.raw_connection()
    }
}

impl<C> Io for Client<C>
where
    C: Io,
{
    type Error = C::Error;
}

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

    pub fn submit(mut self) -> Result<Response<C>, C::Error> {
        self.0.initiate_response()?;

        Ok(Response(self.0))
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

impl<C> Write for Request<C>
where
    C: Connection,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.request().unwrap().write(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.request().unwrap().flush()
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

    pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
        self.0.response().unwrap()
    }

    pub fn release(self) -> C {
        self.0
    }
}

impl<C> Status for Response<C>
where
    C: Connection,
{
    fn status(&self) -> u16 {
        self.0.headers().unwrap().status()
    }

    fn status_message(&self) -> Option<&'_ str> {
        self.0.headers().unwrap().status_message()
    }
}

impl<C> Headers for Response<C>
where
    C: Connection,
{
    fn header(&self, name: &str) -> Option<&'_ str> {
        self.0.headers().unwrap().header(name)
    }
}

impl<C> Io for Response<C>
where
    C: Io,
{
    type Error = C::Error;
}

impl<C> Read for Response<C>
where
    C: Connection,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.response().unwrap().1.read(buf)
    }
}

pub trait Connection: Io {
    type Headers: Status + Headers;

    type Read: Read<Error = Self::Error>;

    type Write: Write<Error = Self::Error>;

    type RawConnectionError: Error;

    type RawConnection: Read<Error = Self::RawConnectionError>
        + Write<Error = Self::RawConnectionError>;

    fn initiate_request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<(), Self::Error>;

    fn request(&mut self) -> Result<&mut Self::Write, Self::Error>;

    fn initiate_response(&mut self) -> Result<(), Self::Error>;

    fn response(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error>;
    fn headers(&self) -> Result<&Self::Headers, Self::Error>;

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

    fn initiate_request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<(), Self::Error> {
        (*self).initiate_request(method, uri, headers)
    }

    fn request(&mut self) -> Result<&mut Self::Write, Self::Error> {
        (*self).request()
    }

    fn initiate_response(&mut self) -> Result<(), Self::Error> {
        (*self).initiate_response()
    }

    fn response(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error> {
        (*self).response()
    }

    fn headers(&self) -> Result<&Self::Headers, Self::Error> {
        (**self).headers()
    }

    fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
        (*self).raw_connection()
    }
}
#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::executor::asynch::{Blocker, RawBlocking, RawTrivialUnblocking};
    use crate::io::{asynch::Read, asynch::Write, Error, Io};

    pub use crate::http::asynch::*;
    pub use crate::http::{Headers, Method, Status};

    #[derive(Debug)]
    pub struct Client<C>(C);

    impl<C> Client<C>
    where
        C: Connection,
    {
        pub const fn wrap(connection: C) -> Self {
            Self(connection)
        }

        pub fn release(self) -> C {
            self.0
        }

        pub async fn get<'a>(&'a mut self, uri: &'a str) -> Result<Request<&'a mut C>, C::Error> {
            self.request(Method::Get, uri, &[]).await
        }

        pub async fn post<'a>(
            &'a mut self,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<Request<&'a mut C>, C::Error> {
            self.request(Method::Post, uri, headers).await
        }

        pub async fn put<'a>(
            &'a mut self,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<Request<&'a mut C>, C::Error> {
            self.request(Method::Put, uri, headers).await
        }

        pub async fn delete<'a>(
            &'a mut self,
            uri: &'a str,
        ) -> Result<Request<&'a mut C>, C::Error> {
            self.request(Method::Delete, uri, &[]).await
        }

        pub async fn request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<Request<&'a mut C>, C::Error> {
            self.0.initiate_request(method, uri, headers).await?;

            Request::wrap(&mut self.0)
        }

        pub fn raw_connection(&mut self) -> Result<&mut C::RawConnection, C::Error> {
            self.0.raw_connection()
        }
    }

    impl<C> Io for Client<C>
    where
        C: Io,
    {
        type Error = C::Error;
    }

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

        pub async fn submit(mut self) -> Result<Response<C>, C::Error> {
            self.0.initiate_response().await?;

            Ok(Response(self.0))
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

    impl<C> Write for Request<C>
    where
        C: Connection,
    {
        type WriteFuture<'b>
        where
            Self: 'b,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn write<'b>(&'b mut self, buf: &'b [u8]) -> Self::WriteFuture<'b> {
            async move { self.0.request()?.write(buf).await }
        }

        type FlushFuture<'b>
        where
            Self: 'b,
        = impl Future<Output = Result<(), Self::Error>>;

        fn flush(&mut self) -> Self::FlushFuture<'_> {
            async move { self.0.request()?.flush().await }
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

        pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
            self.0.response().unwrap()
        }

        pub fn release(self) -> C {
            self.0
        }
    }

    impl<C> Status for Response<C>
    where
        C: Connection,
    {
        fn status(&self) -> u16 {
            self.0.headers().unwrap().status()
        }

        fn status_message(&self) -> Option<&'_ str> {
            self.0.headers().unwrap().status_message()
        }
    }

    impl<C> Headers for Response<C>
    where
        C: Connection,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.0.headers().unwrap().header(name)
        }
    }

    impl<C> Io for Response<C>
    where
        C: Io,
    {
        type Error = C::Error;
    }

    impl<C> Read for Response<C>
    where
        C: Connection,
    {
        type ReadFuture<'b>
        where
            Self: 'b,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn read<'b>(&'b mut self, buf: &'b mut [u8]) -> Self::ReadFuture<'b> {
            async move { self.0.response()?.1.read(buf).await }
        }
    }

    pub trait Connection: Io {
        type Headers: Status + Headers;

        type Read: Read<Error = Self::Error>;

        type Write: Write<Error = Self::Error>;

        type RawConnectionError: Error;

        type RawConnection: Read<Error = Self::RawConnectionError>
            + Write<Error = Self::RawConnectionError>;

        type IntoRequestFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type IntoResponseFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        fn initiate_request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoRequestFuture<'a>;

        fn request(&mut self) -> Result<&mut Self::Write, Self::Error>;

        fn initiate_response(&mut self) -> Self::IntoResponseFuture<'_>;

        fn response(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error>;
        fn headers(&self) -> Result<&Self::Headers, Self::Error>;

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

        type IntoRequestFuture<'a>
        where
            Self: 'a,
        = C::IntoRequestFuture<'a>;

        type IntoResponseFuture<'a>
        where
            Self: 'a,
        = C::IntoResponseFuture<'a>;

        fn initiate_request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoRequestFuture<'a> {
            (*self).initiate_request(method, uri, headers)
        }

        fn request(&mut self) -> Result<&mut Self::Write, Self::Error> {
            (*self).request()
        }

        fn initiate_response(&mut self) -> Self::IntoResponseFuture<'_> {
            (*self).initiate_response()
        }

        fn response(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error> {
            (*self).response()
        }

        fn headers(&self) -> Result<&Self::Headers, Self::Error> {
            (**self).headers()
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            (*self).raw_connection()
        }
    }

    #[derive(Debug)]
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
        B: Blocker,
        C: Connection,
    {
        pub fn new(blocker: B, connection: C) -> Self {
            Self {
                blocker,
                connection,
                lended_read: RawBlocking::new(),
                lended_write: RawBlocking::new(),
                lended_raw: RawBlocking::new(),
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

        fn initiate_request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<(), Self::Error> {
            self.blocker
                .block_on(self.connection.initiate_request(method, uri, headers))?;

            Ok(())
        }

        fn request(&mut self) -> Result<&mut Self::Write, Self::Error> {
            let write = self.connection.request()?;

            self.lended_write.blocker = &self.blocker;
            self.lended_write.api = write;

            Ok(&mut self.lended_write)
        }

        fn initiate_response(&mut self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.connection.initiate_response())?;

            Ok(())
        }

        fn response(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error> {
            let (headers, read) = self.connection.response()?;

            self.lended_read.blocker = &self.blocker;
            self.lended_read.api = read;

            Ok((headers, &mut self.lended_read))
        }

        fn headers(&self) -> Result<&Self::Headers, Self::Error> {
            self.connection.headers()
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            let connection = self.connection.raw_connection()?;

            self.lended_raw.blocker = &self.blocker;
            self.lended_raw.api = connection;

            Ok(&mut self.lended_raw)
        }
    }

    pub struct TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        connection: C,
        lended_read: RawTrivialUnblocking<C::Read>,
        lended_write: RawTrivialUnblocking<C::Write>,
        lended_raw: RawTrivialUnblocking<C::RawConnection>,
    }

    impl<C> TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        pub fn new(connection: C) -> Self {
            Self {
                connection,
                lended_read: RawTrivialUnblocking::new(),
                lended_write: RawTrivialUnblocking::new(),
                lended_raw: RawTrivialUnblocking::new(),
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

        type Read = RawTrivialUnblocking<C::Read>;

        type Write = RawTrivialUnblocking<C::Write>;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = RawTrivialUnblocking<C::RawConnection>;

        type IntoResponseFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>>;

        type IntoRequestFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>>;

        fn initiate_request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoRequestFuture<'a> {
            async move { self.connection.initiate_request(method, uri, headers) }
        }

        fn request(&mut self) -> Result<&mut Self::Write, Self::Error> {
            let write = self.connection.request()?;
            self.lended_write.api = write;

            Ok(&mut self.lended_write)
        }

        fn initiate_response(&mut self) -> Self::IntoResponseFuture<'_> {
            async move { self.connection.initiate_response() }
        }

        fn response(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error> {
            let (headers, read) = self.connection.response()?;

            self.lended_read.api = read;

            Ok((headers, &mut self.lended_read))
        }

        fn headers(&self) -> Result<&Self::Headers, Self::Error> {
            self.connection.headers()
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            let raw_connection = self.connection.raw_connection()?;

            self.lended_raw.api = raw_connection;

            Ok(&mut self.lended_raw)
        }
    }
}
