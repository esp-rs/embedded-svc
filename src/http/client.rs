use crate::io::{Error, Io, Read, Write};

pub use super::{Headers, Method, Status};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Client<C>(C);

impl<C> Client<C>
where
    C: Connection,
{
    pub fn wrap(connection: C) -> Self {
        if connection.is_request_initiated() || connection.is_response_initiated() {
            panic!("connection is not in initial phase");
        }

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

        Ok(Request::wrap(&mut self.0))
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Request<C>(C);

impl<C> Request<C>
where
    C: Connection,
{
    pub fn wrap(connection: C) -> Request<C> {
        if !connection.is_request_initiated() {
            panic!("connection is not in request phase");
        }

        Request(connection)
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

    pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
        self.0.split()
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
        self.0.status()
    }

    fn status_message(&self) -> Option<&'_ str> {
        self.0.status_message()
    }
}

impl<C> Headers for Response<C>
where
    C: Connection,
{
    fn header(&self, name: &str) -> Option<&'_ str> {
        self.0.header(name)
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

pub trait Connection: Status + Headers + Read + Write {
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

    fn is_request_initiated(&self) -> bool;

    fn initiate_response(&mut self) -> Result<(), Self::Error>;

    fn is_response_initiated(&self) -> bool;

    fn split(&mut self) -> (&Self::Headers, &mut Self::Read);

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

    fn is_request_initiated(&self) -> bool {
        (**self).is_request_initiated()
    }

    fn initiate_response(&mut self) -> Result<(), Self::Error> {
        (*self).initiate_response()
    }

    fn is_response_initiated(&self) -> bool {
        (**self).is_response_initiated()
    }

    fn split(&mut self) -> (&Self::Headers, &mut Self::Read) {
        (*self).split()
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
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Client<C>(C);

    impl<C> Client<C>
    where
        C: Connection,
    {
        pub fn wrap(connection: C) -> Self {
            if connection.is_request_initiated() || connection.is_response_initiated() {
                panic!("connection is not in initial phase");
            }

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

            Ok(Request::wrap(&mut self.0))
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
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Request<C>(C);

    impl<C> Request<C>
    where
        C: Connection,
    {
        pub fn wrap(connection: C) -> Request<C> {
            if !connection.is_request_initiated() {
                panic!("connection is not in request phase");
            }

            Request(connection)
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
        = impl Future<Output = Result<usize, Self::Error>> + 'b where Self: 'b;

        fn write<'b>(&'b mut self, buf: &'b [u8]) -> Self::WriteFuture<'b> {
            self.0.write(buf)
        }

        type FlushFuture<'b>
        = impl Future<Output = Result<(), Self::Error>> + 'b where Self: 'b;

        fn flush(&mut self) -> Self::FlushFuture<'_> {
            self.0.flush()
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

        pub fn split(&mut self) -> (&C::Headers, &mut C::Read) {
            self.0.split()
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
            self.0.status()
        }

        fn status_message(&self) -> Option<&'_ str> {
            self.0.status_message()
        }
    }

    impl<C> Headers for Response<C>
    where
        C: Connection,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.0.header(name)
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
        = impl Future<Output = Result<usize, Self::Error>> + 'b where Self: 'b;

        fn read<'b>(&'b mut self, buf: &'b mut [u8]) -> Self::ReadFuture<'b> {
            self.0.read(buf)
        }
    }

    pub trait Connection: Status + Headers + Read + Write {
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

        fn is_request_initiated(&self) -> bool;

        fn initiate_response(&mut self) -> Self::IntoResponseFuture<'_>;

        fn is_response_initiated(&self) -> bool;

        fn split(&mut self) -> (&Self::Headers, &mut Self::Read);

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

        fn is_request_initiated(&self) -> bool {
            (**self).is_request_initiated()
        }

        fn initiate_response(&mut self) -> Self::IntoResponseFuture<'_> {
            (*self).initiate_response()
        }

        fn is_response_initiated(&self) -> bool {
            (**self).is_response_initiated()
        }

        fn split(&mut self) -> (&Self::Headers, &mut Self::Read) {
            (*self).split()
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

    impl<B, C> Status for BlockingConnection<B, C>
    where
        C: Connection,
    {
        fn status(&self) -> u16 {
            self.connection.status()
        }

        fn status_message(&self) -> Option<&'_ str> {
            self.connection.status_message()
        }
    }

    impl<B, C> Headers for BlockingConnection<B, C>
    where
        C: Connection,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.connection.header(name)
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

        fn is_request_initiated(&self) -> bool {
            self.connection.is_request_initiated()
        }

        fn initiate_response(&mut self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.connection.initiate_response())?;

            Ok(())
        }

        fn is_response_initiated(&self) -> bool {
            self.connection.is_response_initiated()
        }

        fn split(&mut self) -> (&Self::Headers, &mut Self::Read) {
            let (headers, read) = self.connection.split();

            self.lended_read.blocker = &self.blocker;
            self.lended_read.api = read;

            (headers, &mut self.lended_read)
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

    impl<C> Status for TrivialUnblockingConnection<C>
    where
        C: super::Connection,
    {
        fn status(&self) -> u16 {
            self.connection.status()
        }

        fn status_message(&self) -> Option<&'_ str> {
            self.connection.status_message()
        }
    }

    impl<C> Headers for TrivialUnblockingConnection<C>
    where
        C: super::Connection,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.connection.header(name)
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
        type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where Self: 'a;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move { self.connection.read(buf) }
        }
    }

    impl<C> Write for TrivialUnblockingConnection<C>
    where
        C: super::Connection,
    {
        type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where Self: 'a;

        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            async move { self.connection.write(buf) }
        }

        type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
        where Self: 'a;

        fn flush(&mut self) -> Self::FlushFuture<'_> {
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
        = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;

        type IntoRequestFuture<'a>
        = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;

        fn initiate_request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoRequestFuture<'a> {
            async move { self.connection.initiate_request(method, uri, headers) }
        }

        fn is_request_initiated(&self) -> bool {
            self.connection.is_request_initiated()
        }

        fn initiate_response(&mut self) -> Self::IntoResponseFuture<'_> {
            async move { self.connection.initiate_response() }
        }

        fn is_response_initiated(&self) -> bool {
            self.connection.is_response_initiated()
        }

        fn split(&mut self) -> (&Self::Headers, &mut Self::Read) {
            let (headers, read) = self.connection.split();

            self.lended_read.api = read;

            (headers, &mut self.lended_read)
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            let raw_connection = self.connection.raw_connection()?;

            self.lended_raw.api = raw_connection;

            Ok(&mut self.lended_raw)
        }
    }
}
