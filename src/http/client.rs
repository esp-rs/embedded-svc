use core::fmt;

use serde::Serialize;

use crate::errors::Errors;
use crate::io::{self, Write};

use super::{Headers, Method, SendHeaders, Status};

pub trait Client: Errors {
    type Request<'a>: Request<'a, Error = Self::Error>
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

#[derive(Debug)]
pub enum SendError<S, W>
where
    S: fmt::Display + fmt::Debug,
    W: fmt::Display + fmt::Debug,
{
    SendError(S),
    WriteError(W),
}

impl<S, W> fmt::Display for SendError<S, W>
where
    S: fmt::Display + fmt::Debug,
    W: fmt::Display + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendError::SendError(s) => write!(f, "Send Error {}", s),
            SendError::WriteError(w) => write!(f, "Write Error {}", w),
        }
    }
}

#[cfg(feature = "std")]
impl<S, W> std::error::Error for SendError<S, W>
where
    S: fmt::Display + fmt::Debug,
    W: fmt::Display + fmt::Debug,
    // TODO
    // where
    //     S: std::error::Error + 'static,
    //     W: std::error::Error + 'static,
{
    // TODO
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     match self {
    //         SendError::SendError(s) => Some(s),
    //         SendError::WriteError(w) => Some(w),
    //     }
    // }
}

pub trait RequestWrite<'a>: io::Write {
    type Response: Response<Error = Self::Error>;

    fn into_response(self) -> Result<Self::Response, Self::Error>;
}

pub trait Request<'a>: SendHeaders<'a> + Errors {
    type Write<'b>: RequestWrite<'b, Error = Self::Error>;

    fn send_bytes(
        self,
        bytes: impl AsRef<[u8]>,
    ) -> Result<<Self::Write<'a> as RequestWrite<'a>>::Response, Self::Error>
    where
        Self: Sized,
    {
        let mut write = self.into_writer(bytes.as_ref().len())?;

        write.do_write_all(bytes.as_ref())?;

        write.into_response()
    }

    fn send_str(
        self,
        s: impl AsRef<str>,
    ) -> Result<<Self::Write<'a> as RequestWrite<'a>>::Response, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(s.as_ref().as_bytes())
    }

    fn send_json<T: Serialize>(
        self,
        o: impl AsRef<T>,
    ) -> Result<
        <Self::Write<'a> as RequestWrite<'a>>::Response,
        SendError<Self::Error, serde_json::Error>,
    >
    where
        Self: Sized,
    {
        let s = serde_json::to_string(o.as_ref()).map_err(SendError::WriteError)?;

        self.send_str(s).map_err(SendError::SendError)
    }

    #[allow(clippy::type_complexity)]
    fn send_reader<R: io::Read>(
        self,
        size: usize,
        read: R,
    ) -> Result<<Self::Write<'a> as RequestWrite<'a>>::Response, SendError<Self::Error, R::Error>>
    where
        Self: Sized,
    {
        let mut write = self.into_writer(size).map_err(SendError::SendError)?;

        io::copy_len(read, &mut write, size as u64).map_err(|e| match e {
            io::CopyError::ReadError(e) => SendError::WriteError(e),
            io::CopyError::WriteError(e) => SendError::SendError(e),
        })?;

        write.into_response().map_err(SendError::SendError)
    }

    fn into_writer(self, size: usize) -> Result<Self::Write<'a>, Self::Error>;

    fn submit(self) -> Result<<Self::Write<'a> as RequestWrite<'a>>::Response, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(&[0_u8; 0])
    }
}

pub trait Response: Status + Headers + Errors {
    type Read<'a>: io::Read<Error = Self::Error>
    where
        Self: 'a;

    fn reader(&self) -> Self::Read<'_>;
}
