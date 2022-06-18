use core::fmt::{self, Debug, Display, Write as _};

use crate::errors::wrap::EitherError;
use crate::io::{self, Io, Read, Write};

use super::{Headers, SendHeaders, SendStatus};

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

struct PrivateData;

pub struct Completion(PrivateData);

impl Completion {
    /// # Safety
    /// This function is marked as unsafe because it is an internal API and is NOT supposed to be called by the user
    pub unsafe fn internal_new() -> Self {
        Self(PrivateData)
    }
}

pub trait ResponseWrite: Write {
    fn complete(self) -> Result<Completion, Self::Error>
    where
        Self: Sized;
}

pub trait Response<const B: usize = 64>: SendStatus + SendHeaders + Io {
    type Write: ResponseWrite<Error = Self::Error>;

    fn send_bytes(self, bytes: &[u8]) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        let mut write = self.into_writer()?;

        write.write_all(bytes.as_ref())?;

        write.complete()
    }

    fn send_str(self, s: &str) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(s.as_bytes())
    }

    #[cfg(feature = "alloc")]
    fn send_json<T>(self, o: &T) -> Result<Completion, EitherError<Self::Error, serde_json::Error>>
    where
        T: serde::Serialize + ?Sized,
        Self: Sized,
    {
        let s = serde_json::to_string(o).map_err(EitherError::E2)?;

        self.send_str(&s).map_err(EitherError::E1)
    }

    fn send_reader<I>(
        self,
        size: Option<usize>,
        read: I,
    ) -> Result<Completion, EitherError<Self::Error, I::Error>>
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

        write.complete().map_err(EitherError::E1)
    }

    fn into_writer(self) -> Result<Self::Write, Self::Error>
    where
        Self: Sized;

    fn submit(self) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(&[0_u8; 0])
    }

    fn redirect(self, location: &str) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        self.header("location", location).submit()
    }
}

pub struct HandlerError(heapless::String<128>);

impl<E> From<E> for HandlerError
where
    E: Debug,
{
    fn from(e: E) -> Self {
        let mut string: heapless::String<128> = "(Unknown)".into();

        let _ = write!(&mut string, "{:?}", e);

        Self(string)
    }
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait Handler<R, S>: Send
where
    R: Request,
    S: Response,
{
    fn handle(&self, req: R, resp: S) -> Result<Completion, HandlerError>;
}

impl<R, S, H> Handler<R, S> for H
where
    R: Request,
    S: Response,
    H: Fn(R, S) -> Result<Completion, HandlerError> + Send + 'static,
{
    fn handle(&self, req: R, resp: S) -> Result<Completion, HandlerError> {
        (self)(req, resp)
    }
}
