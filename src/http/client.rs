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

    pub fn connection(&mut self) -> &mut C {
        &mut self.0
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
        connection.assert_request()?;

        Ok(Request(connection))
    }

    pub fn submit(mut self) -> Result<Response<C>, C::Error> {
        self.0.initiate_response()?;

        Ok(Response(self.0))
    }

    pub fn connection(&mut self) -> &mut C {
        &mut self.0
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
        self.0.write(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

#[derive(Debug)]
pub struct Response<C>(C);

impl<C> Response<C>
where
    C: Connection,
{
    pub fn wrap(connection: C) -> Result<Response<C>, C::Error> {
        connection.headers()?;

        Ok(Response(connection))
    }

    pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
        self.0.split().unwrap()
    }

    pub fn connection(&mut self) -> &mut C {
        &mut self.0
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
        self.0.read(buf)
    }
}

pub trait Connection: Read + Write {
    type Headers: Status + Headers;

    type Read: Read<Error = Self::Error>;

    type RawConnectionError: Error;

    type RawConnection: Read<Error = Self::RawConnectionError>
        + Write<Error = Self::RawConnectionError>;

    fn initiate_request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<(), Self::Error>;

    fn assert_request(&mut self) -> Result<(), Self::Error>;

    fn initiate_response(&mut self) -> Result<(), Self::Error>;

    fn split(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error>;
    fn headers(&self) -> Result<&Self::Headers, Self::Error>;

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

    fn initiate_request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<(), Self::Error> {
        (*self).initiate_request(method, uri, headers)
    }

    fn assert_request(&mut self) -> Result<(), Self::Error> {
        (*self).assert_request()
    }

    fn initiate_response(&mut self) -> Result<(), Self::Error> {
        (*self).initiate_response()
    }

    fn split(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error> {
        (*self).split()
    }

    fn headers(&self) -> Result<&Self::Headers, Self::Error> {
        (**self).headers()
    }

    fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
        (*self).raw_connection()
    }
}

#[cfg(all(feature = "nightly", feature = "experimental"))]
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

        pub fn connection(&mut self) -> &mut C {
            &mut self.0
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
            connection.assert_request()?;

            Ok(Request(connection))
        }

        pub async fn submit(mut self) -> Result<Response<C>, C::Error> {
            self.0.initiate_response().await?;

            Ok(Response(self.0))
        }

        pub fn connection(&mut self) -> &mut C {
            &mut self.0
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
        = impl Future<Output = Result<usize, Self::Error>> where Self: 'b;

        fn write<'b>(&'b mut self, buf: &'b [u8]) -> Self::WriteFuture<'b> {
            async move { self.0.write(buf).await }
        }

        type FlushFuture<'b>
        = impl Future<Output = Result<(), Self::Error>> where Self: 'b;

        fn flush(&mut self) -> Self::FlushFuture<'_> {
            async move { self.0.flush().await }
        }
    }

    #[derive(Debug)]
    pub struct Response<C>(C);

    impl<C> Response<C>
    where
        C: Connection,
    {
        pub fn wrap(connection: C) -> Result<Response<C>, C::Error> {
            connection.headers()?;

            Ok(Response(connection))
        }

        pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
            self.0.split().unwrap()
        }

        pub fn connection(&mut self) -> &mut C {
            &mut self.0
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
        = impl Future<Output = Result<usize, Self::Error>> where Self: 'b;

        fn read<'b>(&'b mut self, buf: &'b mut [u8]) -> Self::ReadFuture<'b> {
            async move { self.0.read(buf).await }
        }
    }

    pub trait Connection: Read + Write {
        type Headers: Status + Headers;

        type Read: Read<Error = Self::Error>;

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

        fn assert_request(&mut self) -> Result<(), Self::Error>;

        fn initiate_response(&mut self) -> Self::IntoResponseFuture<'_>;

        fn split(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error>;
        fn headers(&self) -> Result<&Self::Headers, Self::Error>;

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

        type IntoRequestFuture<'a>
        = C::IntoRequestFuture<'a> where Self: 'a;

        type IntoResponseFuture<'a>
        = C::IntoResponseFuture<'a> where Self: 'a;

        fn initiate_request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoRequestFuture<'a> {
            (*self).initiate_request(method, uri, headers)
        }

        fn assert_request(&mut self) -> Result<(), Self::Error> {
            (*self).assert_request()
        }

        fn initiate_response(&mut self) -> Self::IntoResponseFuture<'_> {
            (*self).initiate_response()
        }

        fn split(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error> {
            (*self).split()
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
        lended_raw: RawBlocking<B, C::RawConnection>,
    }

    impl<B, C> BlockingConnection<B, C>
    where
        B: Blocker,
        C: Connection,
    {
        pub const fn new(blocker: B, connection: C) -> Self {
            Self {
                blocker,
                connection,
                lended_read: RawBlocking::new(),
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

    impl<B, C> super::Read for BlockingConnection<B, C>
    where
        B: Blocker,
        C: Connection,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.blocker.block_on(self.connection.read(buf))
        }
    }

    impl<B, C> super::Write for BlockingConnection<B, C>
    where
        B: Blocker,
        C: Connection,
    {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.blocker.block_on(self.connection.write(buf))
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.connection.flush())
        }
    }

    impl<B, C> super::Connection for BlockingConnection<B, C>
    where
        B: Blocker,
        C: Connection,
    {
        type Headers = C::Headers;

        type Read = RawBlocking<B, C::Read>;

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

        fn assert_request(&mut self) -> Result<(), Self::Error> {
            self.connection.assert_request()
        }

        fn initiate_response(&mut self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.connection.initiate_response())?;

            Ok(())
        }

        fn split(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error> {
            let (headers, read) = self.connection.split()?;

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

    pub struct TrivialUnblockingConnection<C>
    where
        C: super::Connection,
    {
        connection: C,
        lended_read: RawTrivialUnblocking<C::Read>,
        lended_raw: RawTrivialUnblocking<C::RawConnection>,
    }

    impl<C> TrivialUnblockingConnection<C>
    where
        C: super::Connection,
    {
        pub const fn new(connection: C) -> Self {
            Self {
                connection,
                lended_read: RawTrivialUnblocking::new(),
                lended_raw: RawTrivialUnblocking::new(),
            }
        }
    }

    impl<C> Io for TrivialUnblockingConnection<C>
    where
        C: super::Connection,
    {
        type Error = C::Error;
    }

    impl<C> Read for TrivialUnblockingConnection<C>
    where
        C: super::Connection,
    {
        type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
        where Self: 'a;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move { self.connection.read(buf) }
        }
    }

    impl<C> Write for TrivialUnblockingConnection<C>
    where
        C: super::Connection,
    {
        type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
        where Self: 'a;

        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            async move { self.connection.write(buf) }
        }

        type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>>
        where Self: 'a;

        fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
            async move { self.connection.flush() }
        }
    }

    impl<C> Connection for TrivialUnblockingConnection<C>
    where
        C: super::Connection,
    {
        type Headers = C::Headers;

        type Read = RawTrivialUnblocking<C::Read>;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = RawTrivialUnblocking<C::RawConnection>;

        type IntoResponseFuture<'a>
        = impl Future<Output = Result<(), Self::Error>> where Self: 'a;

        type IntoRequestFuture<'a>
        = impl Future<Output = Result<(), Self::Error>> where Self: 'a;

        fn initiate_request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoRequestFuture<'a> {
            async move { self.connection.initiate_request(method, uri, headers) }
        }

        fn assert_request(&mut self) -> Result<(), Self::Error> {
            self.connection.assert_request()
        }

        fn initiate_response(&mut self) -> Self::IntoResponseFuture<'_> {
            async move { self.connection.initiate_response() }
        }

        fn split(&mut self) -> Result<(&Self::Headers, &mut Self::Read), Self::Error> {
            let (headers, read) = self.connection.split()?;

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
