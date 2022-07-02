use crate::io::{Io, Read, Write};

pub use super::{Headers, Method, SendHeaders, Status};

pub trait Client: Io {
    type Request<'a>: Request<Error = Self::Error>
    where
        Self: 'a;

    fn get(&mut self, url: &str) -> Result<Self::Request<'_>, Self::Error> {
        self.request(Method::Get, url)
    }

    fn post(&mut self, url: &str) -> Result<Self::Request<'_>, Self::Error> {
        self.request(Method::Post, url)
    }

    fn put(&mut self, url: &str) -> Result<Self::Request<'_>, Self::Error> {
        self.request(Method::Put, url)
    }

    fn delete(&mut self, url: &str) -> Result<Self::Request<'_>, Self::Error> {
        self.request(Method::Delete, url)
    }

    fn request(&mut self, method: Method, url: &str) -> Result<Self::Request<'_>, Self::Error>;
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

    pub use crate::http::{Headers, Method, SendHeaders, Status};

    pub trait Client: Io {
        type Request<'a>: Request<Error = Self::Error>
        where
            Self: 'a;

        type RequestFuture<'a>: Future<Output = Result<Self::Request<'a>, Self::Error>>
        where
            Self: 'a;

        fn get(&mut self, url: &str) -> Self::RequestFuture<'_> {
            self.request(Method::Get, url)
        }

        fn post(&mut self, url: &str) -> Self::RequestFuture<'_> {
            self.request(Method::Post, url)
        }

        fn put(&mut self, url: &str) -> Self::RequestFuture<'_> {
            self.request(Method::Put, url)
        }

        fn delete(&mut self, url: &str) -> Self::RequestFuture<'_> {
            self.request(Method::Delete, url)
        }

        fn request(&mut self, method: Method, url: &str) -> Self::RequestFuture<'_>;
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
}
