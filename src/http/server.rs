use core::fmt::{self, Debug, Display, Write as _};

use crate::errors::wrap::EitherError;
use crate::io::{self, Io, Read, Write};

pub use super::{Headers, SendHeaders, SendStatus};

pub mod middleware;
pub mod registry;
pub mod session;

pub trait Request: Headers + Io {
    type Read<'b>: Read<Error = Self::Error>
    where
        Self: 'b;

    fn get_request_id(&self) -> &'_ str;

    fn query_string(&self) -> &'_ str;

    fn reader(&mut self) -> Self::Read<'_>;
}

pub trait Response<const B: usize = 64>: SendStatus + SendHeaders + Io {
    type Write: Write<Error = Self::Error>;

    fn send_bytes(self, bytes: &[u8]) -> Result<Self::Write, Self::Error>
    where
        Self: Sized,
    {
        let mut write = self.into_writer()?;

        write.write_all(bytes.as_ref())?;

        Ok(write)
    }

    fn send_str(self, s: &str) -> Result<Self::Write, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(s.as_bytes())
    }

    fn send_reader<I>(
        self,
        size: Option<usize>,
        read: I,
    ) -> Result<Self::Write, EitherError<Self::Error, I::Error>>
    where
        I: Read,
        Self: Sized,
    {
        let mut write = self.into_writer().map_err(EitherError::E1)?;

        if let Some(size) = size {
            io::copy_len::<B, _, _>(read, &mut write, size as u64)
        } else {
            io::copy::<B, _, _>(read, &mut write)
        }
        .map_err(|e| match e {
            EitherError::E1(e) => EitherError::E2(e),
            EitherError::E2(e) => EitherError::E1(e),
        })?;

        Ok(write)
    }

    fn into_writer(self) -> Result<Self::Write, Self::Error>
    where
        Self: Sized;
}

pub struct HandlerError(heapless::String<128>);

impl<E> From<E> for HandlerError
where
    E: Debug,
{
    fn from(e: E) -> Self {
        let mut string: heapless::String<128> = "".into();

        if write!(&mut string, "{:?}", e).is_err() {
            string = "(Error string too big to serve)".into();
        }

        Self(string)
    }
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type HandlerResult = Result<(), HandlerError>;

pub trait Handler<R, S>: Send
where
    R: Request,
    S: Response,
{
    fn handle(&self, req: R, resp: S) -> HandlerResult;
}

impl<R, S, H> Handler<R, S> for H
where
    R: Request,
    S: Response,
    H: Fn(R, S) -> HandlerResult + Send + 'static,
{
    fn handle(&self, req: R, resp: S) -> HandlerResult {
        (self)(req, resp)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::errors::wrap::EitherError;
    use crate::io::{asynch::Read, asynch::Write, Io};

    pub use crate::http::{Headers, SendHeaders, SendStatus};

    pub use super::{HandlerError, HandlerResult};

    pub trait Request: Headers + Io {
        type Read<'b>: Read<Error = Self::Error>
        where
            Self: 'b;

        fn get_request_id(&self) -> &'_ str;

        fn query_string(&self) -> &'_ str;

        fn reader(&mut self) -> Self::Read<'_>;
    }

    pub trait Response<const B: usize = 64>: SendStatus + SendHeaders + Io {
        type Write: Write<Error = Self::Error>;

        type SendFuture<'a>: Future<Output = Result<Self::Write, Self::Error>>;

        type SendBytesFuture<'a>: Future<Output = Result<Self::Write, Self::Error>>
        where
            Self: 'a;

        type SendReaderFuture<E>: Future<Output = Result<Self::Write, EitherError<Self::Error, E>>>;

        type IntoWriterFuture: Future<Output = Result<Self::Write, Self::Error>>;

        fn send_bytes(self, bytes: &[u8]) -> Self::SendBytesFuture<'_>
        where
            Self: Sized;

        fn send_str(self, s: &str) -> Self::SendBytesFuture<'_>
        where
            Self: Sized;

        fn send_reader<I>(self, size: Option<usize>, read: I) -> Self::SendReaderFuture<I::Error>
        where
            I: Read,
            Self: Sized;

        fn into_writer(self) -> Self::IntoWriterFuture
        where
            Self: Sized;
    }

    pub trait Handler<R, S>: Send
    where
        R: Request,
        S: Response,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult>
        where
            Self: 'a,
            R: 'a,
            S: 'a;

        fn handle<'a>(&'a self, req: R, resp: S) -> Self::HandleFuture<'a>
        where
            R: 'a,
            S: 'a;
    }

    // type HFuture = impl Future<Output = HandlerResult;

    // impl<R, S, H> Handler<R, S> for H
    // where
    //     R: Request,
    //     S: Response,
    //     H: Fn(R, S) -> HFuture + 'static,
    // {
    //     type HandleFuture<'a> where Self: 'a = HFuture;

    //     fn handle(&mut self, req: R, resp: S) -> Self::HandleFuture<'_> {
    //         (self)(req, resp)
    //     }
    // }
}
