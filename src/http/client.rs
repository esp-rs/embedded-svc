use core::iter;

use crate::io::{Io, Read, Write};

pub use super::{Headers, Method, Status};

pub trait Client: Io {
    type RequestWrite<'a>: RequestWrite<Error = Self::Error>
    where
        Self: 'a;

    fn get<'a>(&'a mut self, uri: &'a str) -> Result<Self::RequestWrite<'a>, Self::Error> {
        self.request(Method::Get, uri, iter::empty())
    }

    fn post<'a, H>(
        &'a mut self,
        uri: &'a str,
        headers: H,
    ) -> Result<Self::RequestWrite<'a>, Self::Error>
    where
        H: IntoIterator<Item = (&'a str, &'a str)>,
    {
        self.request(Method::Post, uri, headers)
    }

    fn put<'a, H>(
        &'a mut self,
        uri: &'a str,
        headers: H,
    ) -> Result<Self::RequestWrite<'a>, Self::Error>
    where
        H: IntoIterator<Item = (&'a str, &'a str)>,
    {
        self.request(Method::Put, uri, headers)
    }

    fn delete<'a>(&'a mut self, uri: &'a str) -> Result<Self::RequestWrite<'a>, Self::Error> {
        self.request(Method::Delete, uri, iter::empty())
    }

    fn request<'a, H>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: H,
    ) -> Result<Self::RequestWrite<'a>, Self::Error>
    where
        H: IntoIterator<Item = (&'a str, &'a str)>;
}

impl<'c, C> Client for &'c mut C
where
    C: Client,
{
    type RequestWrite<'a>
    where
        Self: 'a,
    = C::RequestWrite<'a>;

    fn request<'a, H>(
        &'a mut self,
        method: Method,
        uri: &'a str,
        headers: H,
    ) -> Result<Self::RequestWrite<'a>, Self::Error>
    where
        H: IntoIterator<Item = (&'a str, &'a str)>,
    {
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
    type Headers<'a>: Status + Headers
    where
        Self: 'a;

    type Read<'a>: Read<Error = Self::Error>
    where
        Self: 'a;

    fn split<'a>(&'a mut self) -> (Self::Headers<'a>, Self::Read<'a>)
    where
        Self: Sized;
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;
    use core::iter::{self, Empty};

    use crate::io::{asynch::Read, asynch::Write, Io};
    use crate::unblocker::asynch::{Blocker, Blocking};

    pub use crate::http::asynch::*;
    pub use crate::http::{Headers, Method, Status};

    pub trait Client: Io {
        type RequestWrite<'a>: RequestWrite<Error = Self::Error>
        where
            Self: 'a;

        type RequestFuture<'a, H>: Future<Output = Result<Self::RequestWrite<'a>, Self::Error>>
        where
            Self: 'a;

        fn get<'a>(
            &'a mut self,
            uri: &'a str,
        ) -> Self::RequestFuture<'a, Empty<(&'a str, &'a str)>> {
            self.request(Method::Get, uri, iter::empty())
        }

        fn post<'a, H>(&'a mut self, uri: &'a str, headers: H) -> Self::RequestFuture<'a, H>
        where
            H: IntoIterator<Item = (&'a str, &'a str)>,
        {
            self.request(Method::Post, uri, headers)
        }

        fn put<'a, H>(&'a mut self, uri: &'a str, headers: H) -> Self::RequestFuture<'a, H>
        where
            H: IntoIterator<Item = (&'a str, &'a str)>,
        {
            self.request(Method::Put, uri, headers)
        }

        fn delete<'a>(
            &'a mut self,
            uri: &'a str,
        ) -> Self::RequestFuture<'a, Empty<(&'a str, &'a str)>> {
            self.request(Method::Delete, uri, iter::empty())
        }

        fn request<'a, H>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: H,
        ) -> Self::RequestFuture<'a, H>
        where
            H: IntoIterator<Item = (&'a str, &'a str)>;
    }

    impl<C> Client for &mut C
    where
        C: Client,
    {
        type RequestWrite<'a>
        where
            Self: 'a,
        = C::RequestWrite<'a>;

        type RequestFuture<'a, H>
        where
            Self: 'a,
        = C::RequestFuture<'a, H>;

        fn request<'a, H>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: H,
        ) -> Self::RequestFuture<'a, H>
        where
            H: IntoIterator<Item = (&'a str, &'a str)>,
        {
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
        type Headers<'a>: Status + Headers
        where
            Self: 'a;

        type Read<'a>: Read<Error = Self::Error>
        where
            Self: 'a;

        fn split<'a>(&'a mut self) -> (Self::Headers<'a>, Self::Read<'a>)
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

        fn request<'a, H>(
            &'a mut self,
            method: Method,
            uri: &'a str,
            headers: H,
        ) -> Result<Self::RequestWrite<'a>, Self::Error>
        where
            H: IntoIterator<Item = (&'a str, &'a str)>,
        {
            let request_write = self.0.block_on(self.1.request(method, uri, headers))?;

            Ok(Blocking::new(&mut self.0, request_write))
        }
    }

    impl<B, W> super::RequestWrite for Blocking<B, W>
    where
        B: Blocker + Clone,
        W: RequestWrite,
    {
        type Response = Blocking<B, W::Response>;

        fn submit(self) -> Result<Self::Response, Self::Error>
        where
            Self: Sized,
        {
            let response = self.0.block_on(self.1.submit())?;

            Ok(Blocking::new(self.0, response))
        }
    }

    impl<B, R> super::Response for Blocking<B, R>
    where
        B: Blocker + Clone,
        R: Response,
    {
        type Headers<'a>
        where
            Self: 'a,
        = R::Headers<'a>;

        type Read<'a>
        where
            Self: 'a,
        = Blocking<B, R::Read<'a>>;

        fn split<'a>(&'a mut self) -> (Self::Headers<'a>, Self::Read<'a>)
        where
            Self: Sized,
        {
            let (headers, body) = self.1.split();

            (headers, Blocking::new(self.0.clone(), body))
        }
    }
}
