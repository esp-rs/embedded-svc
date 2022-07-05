use crate::io::{Io, Read, Write};

pub use super::{Headers, Method, SendHeaders, Status};

pub trait Client: Io {
    type Request<'a>: Request<Error = Self::Error>
    where
        Self: 'a;

    fn get<'a>(&'a mut self, uri: &'a str) -> Result<Self::Request<'a>, Self::Error> {
        self.request(Method::Get, uri)
    }

    fn post<'a>(&'a mut self, uri: &'a str) -> Result<Self::Request<'a>, Self::Error> {
        self.request(Method::Post, uri)
    }

    fn put<'a>(&'a mut self, uri: &'a str) -> Result<Self::Request<'a>, Self::Error> {
        self.request(Method::Put, uri)
    }

    fn delete<'a>(&'a mut self, uri: &'a str) -> Result<Self::Request<'a>, Self::Error> {
        self.request(Method::Delete, uri)
    }

    fn request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
    ) -> Result<Self::Request<'a>, Self::Error>;
}

impl<'c, C> Client for &'c mut C
where
    C: Client,
{
    type Request<'a>
    where
        Self: 'a,
    = C::Request<'a>;

    fn request<'a>(
        &'a mut self,
        method: Method,
        uri: &'a str,
    ) -> Result<Self::Request<'a>, Self::Error> {
        (*self).request(method, uri)
    }
}

pub trait RequestWrite: Write {
    type Response: Response<Error = Self::Error>;

    fn submit(self) -> Result<Self::Response, Self::Error>
    where
        Self: Sized;
}

pub trait Request: SendHeaders + Io {
    type Write: RequestWrite<Error = Self::Error>;

    fn into_writer(self) -> Result<Self::Write, Self::Error>
    where
        Self: Sized;

    fn submit(self) -> Result<<Self::Write as RequestWrite>::Response, Self::Error>
    where
        Self: Sized;
}

pub trait Response: Status + Headers + Read {
    type Headers: Status + Headers;

    type Body: Read<Error = Self::Error>;

    fn split(self) -> (Self::Headers, Self::Body)
    where
        Self: Sized;
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::io::{asynch::Read, asynch::Write, Io};
    use crate::unblocker::asynch::{Blocker, Blocking};

    pub use crate::http::asynch::*;
    pub use crate::http::{Headers, Method, SendHeaders, Status};

    pub trait Client: Io {
        type Request<'a>: Request<Error = Self::Error>
        where
            Self: 'a;

        type RequestFuture<'a>: Future<Output = Result<Self::Request<'a>, Self::Error>>
        where
            Self: 'a;

        fn get<'a>(&'a mut self, uri: &'a str) -> Self::RequestFuture<'a> {
            self.request(Method::Get, uri)
        }

        fn post<'a>(&'a mut self, uri: &'a str) -> Self::RequestFuture<'a> {
            self.request(Method::Post, uri)
        }

        fn put<'a>(&'a mut self, uri: &'a str) -> Self::RequestFuture<'a> {
            self.request(Method::Put, uri)
        }

        fn delete<'a>(&'a mut self, uri: &'a str) -> Self::RequestFuture<'a> {
            self.request(Method::Delete, uri)
        }

        fn request<'a>(&'a mut self, method: Method, uri: &'a str) -> Self::RequestFuture<'a>;
    }

    impl<C> Client for &mut C
    where
        C: Client,
    {
        type Request<'a>
        where
            Self: 'a,
        = C::Request<'a>;

        type RequestFuture<'a>
        where
            Self: 'a,
        = C::RequestFuture<'a>;

        fn request<'a>(&'a mut self, method: Method, uri: &'a str) -> Self::RequestFuture<'a> {
            (*self).request(method, uri)
        }
    }

    pub trait RequestWrite: Write {
        type Response: Response<Error = Self::Error>;

        type IntoResponseFuture: Future<Output = Result<Self::Response, Self::Error>>;

        fn into_response(self) -> Self::IntoResponseFuture
        where
            Self: Sized;
    }

    pub trait Request: SendHeaders + Io {
        type Write: RequestWrite<Error = Self::Error>;

        type IntoWriterFuture: Future<Output = Result<Self::Write, Self::Error>>;

        type SubmitFuture: Future<
            Output = Result<<Self::Write as RequestWrite>::Response, Self::Error>,
        >;

        fn into_writer(self) -> Self::IntoWriterFuture
        where
            Self: Sized;

        fn submit(self) -> Self::SubmitFuture
        where
            Self: Sized;
    }

    pub trait Response: Status + Headers + Read {
        type Headers: Status + Headers;

        type Body: Read<Error = Self::Error>;

        fn split(self) -> (Self::Headers, Self::Body)
        where
            Self: Sized;
    }

    impl<B, C> super::Client for Blocking<B, C>
    where
        B: Blocker,
        C: Client,
    {
        type Request<'a>
        where
            Self: 'a,
        = Blocking<&'a B, C::Request<'a>>;

        fn request<'a>(
            &'a mut self,
            method: Method,
            uri: &'a str,
        ) -> Result<Self::Request<'a>, Self::Error> {
            let request = self.0.block_on(self.1.request(method, uri))?;

            Ok(Blocking::new(&mut self.0, request))
        }
    }

    impl<B, R> super::Request for Blocking<B, R>
    where
        B: Blocker,
        R: Request,
    {
        type Write = Blocking<B, R::Write>;

        fn into_writer(self) -> Result<Self::Write, Self::Error>
        where
            Self: Sized,
        {
            let writer = self.0.block_on(self.1.into_writer())?;

            Ok(Blocking::new(self.0, writer))
        }

        fn submit(self) -> Result<<Self::Write as super::RequestWrite>::Response, Self::Error>
        where
            Self: Sized,
        {
            todo!()
        }
    }

    impl<B, W> super::RequestWrite for Blocking<B, W>
    where
        B: Blocker,
        W: RequestWrite,
    {
        type Response = Blocking<B, W::Response>;

        fn submit(self) -> Result<Self::Response, Self::Error>
        where
            Self: Sized,
        {
            let response = self.0.block_on(self.1.into_response())?;

            Ok(Blocking::new(self.0, response))
        }
    }

    impl<B, R> super::Response for Blocking<B, R>
    where
        B: Blocker,
        R: Response,
    {
        type Headers = R::Headers;

        type Body = Blocking<B, R::Body>;

        fn split(self) -> (Self::Headers, Self::Body)
        where
            Self: Sized,
        {
            let (headers, body) = self.1.split();

            (headers, Blocking::new(self.0, body))
        }
    }
}
