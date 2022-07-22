use crate::io::{Error, Io, Read, Write};

pub use super::{Headers, Method, Status};

pub trait Client: Io {
    type RequestWrite<'a>: RequestWrite<Error = Self::Error>
    where
        Self: 'a;

    type RawConnectionError: Error;

    type RawConnection: Read<Error = Self::RawConnectionError>
        + Write<Error = Self::RawConnectionError>;

    fn get<'a>(&'a mut self, uri: &'a str) -> Result<Self::RequestWrite<'a>, Self::Error> {
        self.request(Method::Get, uri, &[])
    }

    fn post<'a>(
        &'a mut self,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Self::RequestWrite<'a>, Self::Error> {
        self.request(Method::Post, uri, headers)
    }

    fn put<'a>(
        &'a mut self,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Self::RequestWrite<'a>, Self::Error> {
        self.request(Method::Put, uri, headers)
    }

    fn delete<'a>(&'a mut self, uri: &'a str) -> Result<Self::RequestWrite<'a>, Self::Error> {
        self.request(Method::Delete, uri, &[])
    }

    fn request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Self::RequestWrite<'a>, Self::Error>;

    fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error>;
}

impl<'c, C> Client for &'c mut C
where
    C: Client,
{
    type RequestWrite<'a>
    where
        Self: 'a,
    = C::RequestWrite<'a>;

    type RawConnectionError = C::RawConnectionError;

    type RawConnection = C::RawConnection;

    fn request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Self::RequestWrite<'a>, Self::Error> {
        (*self).request(method, uri, headers)
    }

    fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
        (*self).raw_connection()
    }
}

pub trait RequestWrite: Write {
    type Response: Response<Error = Self::Error>;

    fn submit(self) -> Result<Self::Response, Self::Error>
    where
        Self: Sized;
}

pub trait Response: Status + Headers + Read {
    type Headers: Status + Headers;

    type Read: Read<Error = Self::Error>;

    fn split<'a>(&'a mut self) -> (&'a Self::Headers, &'a mut Self::Read);
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::executor::asynch::{Blocker, Blocking, RawBlocking, RawTrivialAsync, TrivialAsync};
    use crate::io::{asynch::Read, asynch::Write, Error, Io, Read as _};

    pub use crate::http::asynch::*;
    pub use crate::http::{Headers, Method, Status};

    pub trait Client: Io {
        type RequestWrite<'a>: RequestWrite<Error = Self::Error>
        where
            Self: 'a;

        type RawConnectionError: Error;

        type RawConnection: Read<Error = Self::RawConnectionError>
            + Write<Error = Self::RawConnectionError>;

        type RequestFuture<'a>: Future<Output = Result<Self::RequestWrite<'a>, Self::Error>>
        where
            Self: 'a;

        fn get<'a>(&'a mut self, uri: &'a str) -> Self::RequestFuture<'a> {
            self.request(Method::Get, uri, &[])
        }

        fn post<'a>(
            &'a mut self,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::RequestFuture<'a> {
            self.request(Method::Post, uri, headers)
        }

        fn put<'a>(
            &'a mut self,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::RequestFuture<'a> {
            self.request(Method::Put, uri, headers)
        }

        fn delete<'a>(&'a mut self, uri: &'a str) -> Self::RequestFuture<'a> {
            self.request(Method::Delete, uri, &[])
        }

        fn request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::RequestFuture<'a>;

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error>;
    }

    impl<C> Client for &mut C
    where
        C: Client,
    {
        type RequestWrite<'a>
        where
            Self: 'a,
        = C::RequestWrite<'a>;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = C::RawConnection;

        type RequestFuture<'a>
        where
            Self: 'a,
        = C::RequestFuture<'a>;

        fn request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::RequestFuture<'a> {
            (*self).request(method, uri, headers)
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            (*self).raw_connection()
        }
    }

    pub trait RequestWrite: Write {
        type Response: Response<Error = Self::Error>;

        type IntoResponseFuture: Future<Output = Result<Self::Response, Self::Error>>;

        fn submit(self) -> Self::IntoResponseFuture
        where
            Self: Sized;
    }

    pub trait Response: Status + Headers + Read {
        type Headers: Status + Headers;

        type Read: Read<Error = Self::Error>;

        fn split<'a>(&'a mut self) -> (&'a Self::Headers, &'a mut Self::Read);
    }

    pub struct BlockingClient<B, C>
    where
        C: Client,
    {
        blocker: B,
        client: C,
        lended_raw: RawBlocking<B, C::RawConnection>,
    }

    impl<B, C> BlockingClient<B, C>
    where
        B: Blocker,
        C: Client,
    {
        pub fn new(blocker: B, client: C) -> Self {
            Self {
                blocker,
                client,
                lended_raw: unsafe { RawBlocking::new() },
            }
        }
    }

    impl<B, C> Io for BlockingClient<B, C>
    where
        C: Client,
    {
        type Error = C::Error;
    }

    impl<B, C> super::Client for BlockingClient<B, C>
    where
        B: Blocker,
        C: Client,
    {
        type RequestWrite<'a>
        where
            Self: 'a,
        = Blocking<&'a B, C::RequestWrite<'a>>;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = RawBlocking<B, C::RawConnection>;

        fn request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<Self::RequestWrite<'a>, Self::Error> {
            let request_write = self
                .blocker
                .block_on(self.client.request(method, uri, headers))?;

            Ok(Blocking::new(&mut self.blocker, request_write))
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            let connection = self.client.raw_connection()?;

            self.lended_raw.blocker = &self.blocker;
            self.lended_raw.api = connection;

            Ok(&mut self.lended_raw)
        }
    }

    impl<B, W> super::RequestWrite for Blocking<B, W>
    where
        B: Blocker + Clone,
        W: RequestWrite,
    {
        type Response = BlockingResponse<B, W::Response>;

        fn submit(self) -> Result<Self::Response, Self::Error>
        where
            Self: Sized,
        {
            let response = self.blocker.block_on(self.api.submit())?;

            Ok(BlockingResponse::new(self.blocker, response))
        }
    }

    pub struct BlockingResponse<B, R>
    where
        R: Response,
    {
        blocker: B,
        response: R,
        lended_read: RawBlocking<B, R::Read>,
    }

    impl<B, R> BlockingResponse<B, R>
    where
        R: Response,
    {
        fn new(blocker: B, response: R) -> Self {
            Self {
                blocker,
                response,
                lended_read: unsafe { RawBlocking::new() },
            }
        }

        pub fn blocker(&self) -> &B {
            &self.blocker
        }

        pub fn api(&self) -> &R {
            &self.response
        }

        pub fn api_mut(&mut self) -> &mut R {
            &mut self.response
        }
    }

    impl<B, R> super::Status for BlockingResponse<B, R>
    where
        R: Response,
    {
        fn status(&self) -> u16 {
            self.response.status()
        }

        fn status_message(&self) -> Option<&'_ str> {
            self.response.status_message()
        }
    }

    impl<B, R> super::Headers for BlockingResponse<B, R>
    where
        R: Response,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.response.header(name)
        }
    }

    impl<B, R> Io for BlockingResponse<B, R>
    where
        R: Response,
    {
        type Error = R::Error;
    }

    impl<B, R> super::Read for BlockingResponse<B, R>
    where
        B: Blocker,
        R: Response,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.blocker.block_on(self.response.read(buf))
        }
    }

    impl<B, R> super::Response for BlockingResponse<B, R>
    where
        B: Blocker + Clone,
        R: Response,
    {
        type Headers = R::Headers;

        type Read = RawBlocking<B, R::Read>;

        fn split<'a>(&'a mut self) -> (&'a Self::Headers, &'a mut Self::Read) {
            let (headers, body) = self.response.split();

            self.lended_read.blocker = &self.blocker;
            self.lended_read.api = body;

            (headers, &mut self.lended_read)
        }
    }

    pub struct TrivialAsyncClient<C>
    where
        C: super::Client,
    {
        client: C,
        lended_raw: RawTrivialAsync<C::RawConnection>,
    }

    impl<C> TrivialAsyncClient<C>
    where
        C: super::Client,
    {
        pub fn new(client: C) -> Self {
            Self {
                client,
                lended_raw: unsafe { RawTrivialAsync::new() },
            }
        }
    }

    impl<C> Io for TrivialAsyncClient<C>
    where
        C: super::Client,
    {
        type Error = C::Error;
    }

    impl<C> Client for TrivialAsyncClient<C>
    where
        C: super::Client,
    {
        type RequestWrite<'a>
        where
            Self: 'a,
        = TrivialAsync<C::RequestWrite<'a>>;

        type RawConnectionError = C::RawConnectionError;

        type RawConnection = RawTrivialAsync<C::RawConnection>;

        type RequestFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<Self::RequestWrite<'a>, Self::Error>>;

        fn request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::RequestFuture<'a> {
            async move {
                let request_write = self.client.request(method, uri, headers)?;

                Ok(TrivialAsync::new(request_write))
            }
        }

        fn raw_connection(&mut self) -> Result<&mut Self::RawConnection, Self::Error> {
            self.lended_raw.api = self.client.raw_connection()?;

            Ok(&mut self.lended_raw)
        }
    }

    impl<W> RequestWrite for TrivialAsync<W>
    where
        W: super::RequestWrite,
    {
        type Response = TrivialAsyncResponse<W::Response>;

        type IntoResponseFuture = impl Future<Output = Result<Self::Response, Self::Error>>;

        fn submit(self) -> Self::IntoResponseFuture
        where
            Self: Sized,
        {
            async move { Ok(TrivialAsyncResponse::new(self.api.submit()?)) }
        }
    }

    pub struct TrivialAsyncResponse<R>
    where
        R: super::Response,
    {
        response: R,
        lended_io: TrivialAsyncIo<R>,
    }

    impl<R> TrivialAsyncResponse<R>
    where
        R: super::Response,
    {
        const fn new(response: R) -> Self {
            Self {
                response,
                lended_io: TrivialAsyncIo::None,
            }
        }

        pub fn api(&self) -> &R {
            &self.response
        }

        pub fn api_mut(&mut self) -> &mut R {
            &mut self.response
        }
    }

    impl<R> super::Status for TrivialAsyncResponse<R>
    where
        R: super::Response,
    {
        fn status(&self) -> u16 {
            self.response.status()
        }

        fn status_message(&self) -> Option<&'_ str> {
            self.response.status_message()
        }
    }

    impl<R> super::Headers for TrivialAsyncResponse<R>
    where
        R: super::Response,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.response.header(name)
        }
    }

    impl<R> Io for TrivialAsyncResponse<R>
    where
        R: super::Response,
    {
        type Error = R::Error;
    }

    impl<R> Read for TrivialAsyncResponse<R>
    where
        R: super::Response,
    {
        type ReadFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move { self.response.read(buf) }
        }
    }

    impl<R> Response for TrivialAsyncResponse<R>
    where
        R: super::Response,
    {
        type Headers = R::Headers;

        type Read = TrivialAsyncIo<R>;

        fn split<'a>(&'a mut self) -> (&'a Self::Headers, &'a mut Self::Read) {
            let (headers, body) = self.response.split();

            self.lended_io = TrivialAsyncIo::Reader(body);

            (headers, &mut self.lended_io)
        }
    }

    pub enum TrivialAsyncIo<R>
    where
        R: super::Response,
    {
        None,
        Reader(*mut R::Read),
    }

    impl<R> Io for TrivialAsyncIo<R>
    where
        R: super::Response,
    {
        type Error = R::Error;
    }

    impl<R> Read for TrivialAsyncIo<R>
    where
        R: super::Response,
    {
        type ReadFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move {
                match self {
                    Self::None => panic!(),
                    Self::Reader(r) => unsafe { r.as_mut().unwrap() }.read(buf),
                }
            }
        }
    }
}
