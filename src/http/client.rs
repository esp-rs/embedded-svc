use crate::errors::wrap::EitherError;
use crate::io::{self, Io, Read, Write};

use super::{Headers, Method, SendHeaders, Status};

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

    fn into_response(self) -> Result<Self::Response, Self::Error>;
}

pub trait Request: SendHeaders + Io {
    type Write: RequestWrite<Error = Self::Error>;

    fn send_bytes(
        self,
        bytes: &[u8],
    ) -> Result<<Self::Write as RequestWrite>::Response, Self::Error>
    where
        Self: Sized,
    {
        let mut write = self.into_writer(bytes.as_ref().len())?;

        write.write_all(bytes)?;

        write.into_response()
    }

    fn send_str(self, s: &str) -> Result<<Self::Write as RequestWrite>::Response, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(s.as_bytes())
    }

    #[allow(clippy::type_complexity)]
    fn send_reader<R: Read>(
        self,
        size: usize,
        read: R,
    ) -> Result<<Self::Write as RequestWrite>::Response, EitherError<Self::Error, R::Error>>
    where
        Self: Sized,
    {
        let mut write = self.into_writer(size).map_err(EitherError::E1)?;

        io::copy_len::<64, _, _>(read, &mut write, size as u64).map_err(|e| match e {
            EitherError::E1(e) => EitherError::E2(e),
            EitherError::E2(e) => EitherError::E1(e),
        })?;

        write.into_response().map_err(EitherError::E1)
    }

    fn into_writer(self, size: usize) -> Result<Self::Write, Self::Error>;

    fn submit(self) -> Result<<Self::Write as RequestWrite>::Response, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(&[0_u8; 0])
    }
}

pub trait Response: Status + Headers + Io {
    type Read<'a>: io::Read<Error = Self::Error>
    where
        Self: 'a;

    fn reader(&mut self) -> Self::Read<'_>;
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::io::{self, asynch::Read, asynch::Write, Io};

    use super::{Headers, Method, SendHeaders, Status};

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

        fn into_response(self) -> Self::IntoResponseFuture;
    }

    pub trait Request: SendHeaders + Io {
        type Write: RequestWrite<Error = Self::Error>;

        type SendFuture<'a>: Future<
            Output = Result<<Self::Write as RequestWrite>::Response, Self::Error>,
        >;

        type SendBytesFuture<'a>: Future<
            Output = Result<<Self::Write as RequestWrite>::Response, Self::Error>,
        >
        where
            Self: 'a;

        type SendReaderFuture: Future<
            Output = Result<<Self::Write as RequestWrite>::Response, Self::Error>,
        >;

        type IntoWriterFuture: Future<
            Output = Result<<Self::Write as RequestWrite>::Response, Self::Error>,
        >;

        type SubmitFuture: Future<
            Output = Result<<Self::Write as RequestWrite>::Response, Self::Error>,
        >;

        fn send_bytes<'a>(self, bytes: &'a [u8]) -> Self::SendBytesFuture<'a>
        where
            Self: Sized + 'a;

        fn send_str<'a>(self, s: &'a str) -> Self::SendBytesFuture<'a>
        where
            Self: Sized + 'a,
        {
            self.send_bytes(s.as_bytes())
        }

        #[allow(clippy::type_complexity)]
        fn send_reader<R: Read>(self, size: usize, read: R) -> Self::SendReaderFuture
        where
            Self: Sized;

        fn into_writer(self, size: usize) -> Self::IntoWriterFuture
        where
            Self: Sized;

        fn submit(self) -> Self::SubmitFuture
        where
            Self: Sized;
    }

    pub trait Response: Status + Headers + Io {
        type Read<'a>: io::asynch::Read<Error = Self::Error>
        where
            Self: 'a;

        fn reader(&mut self) -> Self::Read<'_>;
    }
}
