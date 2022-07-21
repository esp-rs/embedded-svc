use crate::io::{Io, Read, Write};

pub use super::{Headers, Method, Status};

pub trait Client: Io {
    type RequestWrite<'a>: RequestWrite<Error = Self::Error>
    where
        Self: 'a;

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
}

impl<'c, C> Client for &'c mut C
where
    C: Client,
{
    type RequestWrite<'a>
    where
        Self: 'a,
    = C::RequestWrite<'a>;

    fn request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Self::RequestWrite<'a>, Self::Error> {
        (*self).request(method, uri, headers)
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

    fn split<'a>(&'a mut self) -> (&'a Self::Headers, &'a mut Self::Read)
    where
        Self: Sized;
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::executor::asynch::{Blocker, Blocking, TrivialAsync};
    use crate::io::{asynch::Read, asynch::Write, Io, Read as _};

    pub use crate::http::asynch::*;
    pub use crate::http::{Headers, Method, Status};

    pub trait Client: Io {
        type RequestWrite<'a>: RequestWrite<Error = Self::Error>
        where
            Self: 'a;

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
    }

    impl<C> Client for &mut C
    where
        C: Client,
    {
        type RequestWrite<'a>
        where
            Self: 'a,
        = C::RequestWrite<'a>;

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

        fn split<'a>(&'a mut self) -> (&'a Self::Headers, &'a mut Self::Read)
        where
            Self: Sized;
    }

    impl<B, C> super::Client for Blocking<B, C>
    where
        B: Blocker,
        C: Client,
    {
        type RequestWrite<'a>
        where
            Self: 'a,
        = Blocking<&'a B, C::RequestWrite<'a>>;

        fn request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<Self::RequestWrite<'a>, Self::Error> {
            let request_write = self.0.block_on(self.1.request(method, uri, headers))?;

            Ok(Blocking::new(&mut self.0, request_write))
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
            let response = self.0.block_on(self.1.submit())?;

            Ok(BlockingResponse::new(self.0, response))
        }
    }

    pub struct BlockingResponse<B, R>
    where
        R: Response,
    {
        blocker: B,
        response: R,
        lended_io: BlockingIo<B, R>,
    }

    impl<B, R> BlockingResponse<B, R>
    where
        R: Response,
    {
        const fn new(blocker: B, response: R) -> Self {
            Self {
                blocker,
                response,
                lended_io: BlockingIo::None,
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

        type Read = BlockingIo<B, R>;

        fn split<'a>(&'a mut self) -> (&'a Self::Headers, &'a mut Self::Read)
        where
            Self: Sized,
        {
            let (headers, body) = self.response.split();

            self.lended_io = BlockingIo::Reader(Blocking::new(self.blocker.clone(), body));

            (headers, &mut self.lended_io)
        }
    }

    pub enum BlockingIo<B, R>
    where
        R: Response,
    {
        None,
        Reader(Blocking<B, *mut R::Read>),
    }

    impl<B, R> Io for BlockingIo<B, R>
    where
        R: Response,
    {
        type Error = R::Error;
    }

    impl<B, R> crate::io::Read for BlockingIo<B, R>
    where
        B: Blocker,
        R: Response,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            match self {
                Self::None => panic!(),
                Self::Reader(r) => r.0.block_on(unsafe { r.1.as_mut().unwrap() }.read(buf)),
            }
        }
    }

    impl<C> Client for TrivialAsync<C>
    where
        C: super::Client,
    {
        type RequestWrite<'a>
        where
            Self: 'a,
        = TrivialAsync<C::RequestWrite<'a>>;

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
                let request_write = self.1.request(method, uri, headers)?;

                Ok(TrivialAsync::new_async(request_write))
            }
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
            async move { Ok(TrivialAsyncResponse::new(self.1.submit()?)) }
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

        fn split<'a>(&'a mut self) -> (&'a Self::Headers, &'a mut Self::Read)
        where
            Self: Sized,
        {
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
