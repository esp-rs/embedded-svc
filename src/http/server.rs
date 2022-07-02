use core::fmt::{self, Debug, Display, Write as _};

use crate::io::{Io, Read, Write};

pub use super::{Headers, Query, RequestId, SendHeaders, SendStatus};

pub mod middleware;
pub mod registry;

pub trait Request: Query + RequestId + Headers + Read {
    type Headers: Query + RequestId + Headers;

    type Body: Read<Error = Self::Error>;

    fn split(self) -> (Self::Headers, Self::Body);
}

pub trait Response<const B: usize = 64>: SendStatus + SendHeaders + Io {
    type Write: Write<Error = Self::Error>;

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

    use crate::io::{asynch::Read, asynch::Write, Io};

    pub use crate::http::{Headers, Query, RequestId, SendHeaders, SendStatus};

    pub use super::{HandlerError, HandlerResult};

    pub trait Request: Query + RequestId + Headers + Read {
        type Headers: Query + RequestId + Headers;

        type Body: Read<Error = Self::Error>;

        fn split(self) -> (Self::Headers, Self::Body);
    }

    pub trait Response: SendStatus + SendHeaders + Io {
        type Write: Write<Error = Self::Error>;

        type IntoWriterFuture: Future<Output = Result<Self::Write, Self::Error>>;

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
            Self: 'a;

        fn handle(&self, req: R, resp: S) -> Self::HandleFuture<'_>;
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
