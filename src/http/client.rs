use crate::errors::{EitherError, Errors};
use crate::io::{copy_len, Read, Write};

use super::{Headers, Method, SendHeaders, Status};

pub trait Client: Errors {
    type Request<'a>: Request<Error = Self::Error>
    where
        Self: 'a;

    fn get(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
        self.request(Method::Get, url)
    }

    fn post(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
        self.request(Method::Post, url)
    }

    fn put(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
        self.request(Method::Put, url)
    }

    fn delete(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
        self.request(Method::Delete, url)
    }

    fn request(
        &mut self,
        method: Method,
        url: impl AsRef<str>,
    ) -> Result<Self::Request<'_>, Self::Error>;
}

pub trait RequestWrite: Write {
    type Response: Response<Error = Self::Error>;

    fn into_response(self) -> Result<Self::Response, Self::Error>;
}

pub trait Request: SendHeaders + Errors {
    type Write: RequestWrite<Error = Self::Error>;

    fn send_bytes(
        self,
        bytes: &[u8],
    ) -> Result<<Self::Write as RequestWrite>::Response, Self::Error>
    where
        Self: Sized,
    {
        let mut write = self.into_writer(bytes.as_ref().len())?;

        write.write_all(bytes.as_ref())?;

        write.into_response()
    }

    fn send_str(self, s: &str) -> Result<<Self::Write as RequestWrite>::Response, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(s.as_bytes())
    }

    #[cfg(feature = "alloc")]
    fn send_json<T>(
        self,
        o: &T,
    ) -> Result<<Self::Write as RequestWrite>::Response, EitherError<Self::Error, serde_json::Error>>
    where
        T: serde::Serialize,
        Self: Sized,
    {
        let s = serde_json::to_string(o).map_err(EitherError::Second)?;

        self.send_str(&s).map_err(EitherError::First)
    }

    #[allow(clippy::type_complexity)]
    fn send_reader<R>(
        self,
        size: usize,
        read: R,
    ) -> Result<<Self::Write as RequestWrite>::Response, EitherError<Self::Error, R::Error>>
    where
        R: Read,
        Self: Sized,
    {
        let mut write = self.into_writer(size).map_err(EitherError::First)?;

        copy_len::<64, _, _>(read, &mut write, size as u64).map_err(|e| match e {
            EitherError::First(e) => EitherError::Second(e),
            EitherError::Second(e) => EitherError::First(e),
        })?;

        write.into_response().map_err(EitherError::First)
    }

    fn into_writer(self, size: usize) -> Result<Self::Write, Self::Error>
    where
        Self: Sized;

    fn submit(self) -> Result<<Self::Write as RequestWrite>::Response, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(&[0_u8; 0])
    }
}

pub trait Response: Status + Headers + Errors {
    type Read<'a>: Read<Error = Self::Error>
    where
        Self: 'a;

    fn reader(&self) -> Self::Read<'_>;
}

pub mod asyncs {
    use core::future::Future;

    use crate::errors::{EitherError, Errors};
    use crate::io::asyncs::{Read, Write};

    use super::{Headers, Method, SendHeaders, Status};

    pub trait Client: Errors {
        type Request<'a>: Request<Error = Self::Error>
        where
            Self: 'a;

        fn get(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
            self.request(Method::Get, url)
        }

        fn post(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
            self.request(Method::Post, url)
        }

        fn put(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
            self.request(Method::Put, url)
        }

        fn delete(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
            self.request(Method::Delete, url)
        }

        fn request(
            &mut self,
            method: Method,
            url: impl AsRef<str>,
        ) -> Result<Self::Request<'_>, Self::Error>;
    }

    pub trait RequestWrite: Write {
        type Response: Response<Error = Self::Error>;

        fn into_response(self) -> Result<Self::Response, Self::Error>;
    }

    pub trait Request: SendHeaders + Errors {
        type Write: RequestWrite<Error = Self::Error>;

        type SendFuture: Future<
            Output = Result<<Self::Write as RequestWrite>::Response, Self::Error>,
        >;

        #[cfg(feature = "alloc")]
        type SendJsonFuture: Future<
            Output = Result<
                <Self::Write as RequestWrite>::Response,
                EitherError<Self::Error, serde_json::Error>,
            >,
        >;

        type SendReaderFuture<E>: Future<
            Output = Result<<Self::Write as RequestWrite>::Response, EitherError<Self::Error, E>>,
        >;

        type IntoWriterFuture: Future<Output = Result<Self::Write, Self::Error>>;

        fn send_bytes(self, bytes: &[u8]) -> Self::SendFuture
        where
            Self: Sized;

        fn send_str(self, s: &str) -> Self::SendFuture
        where
            Self: Sized;

        #[cfg(feature = "alloc")]
        fn send_json<T>(self, o: &T) -> Self::SendJsonFuture
        where
            T: serde::Serialize,
            Self: Sized;

        #[allow(clippy::type_complexity)]
        fn send_reader<R>(self, size: usize, read: R) -> Self::SendReaderFuture<R::Error>
        where
            R: Read,
            Self: Sized;

        fn into_writer(self, size: usize) -> Self::IntoWriterFuture
        where
            Self: Sized;

        fn submit(self) -> Self::SendFuture
        where
            Self: Sized;
    }

    pub trait Response: Status + Headers + Errors {
        type Read<'a>: Read<Error = Self::Error>
        where
            Self: 'a;

        fn reader(&self) -> Self::Read<'_>;
    }
}
