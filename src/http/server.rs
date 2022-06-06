use core::fmt;

use crate::errors::wrap::EitherError;
use crate::io::{self, Io, Read, Write};
use crate::storage::{DynStorage, Storage};

use super::{Headers, Method, SendHeaders, SendStatus};

pub mod attr;
pub mod middleware;
pub mod registry;
//pub mod session;

#[cfg(feature = "alloc")]
pub use response_data::*;

#[derive(Debug)]
pub enum SessionError {
    MissingError,
    TimeoutError,
    InvalidatedError,
    MaxSessiuonsReachedError,
    SerdeError, // TODO
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::MissingError => write!(f, "No session"),
            SessionError::TimeoutError => write!(f, "Session timed out"),
            SessionError::InvalidatedError => write!(f, "Session invalidated"),
            SessionError::MaxSessiuonsReachedError => write!(f, "Max number of sessions reached"),
            SessionError::SerdeError => write!(f, "Serde error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SessionError {
    // TODO
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     match self {
    //         SendError::SendError(s) => Some(s),
    //         SendError::WriteError(w) => Some(w),
    //     }
    // }
}

pub trait Session: Storage<Error = SessionError> {
    fn create_if_invalid(&mut self) -> Result<&mut Self, SessionError>;

    fn get_error(&self) -> Option<SessionError>;

    fn is_valid(&self) -> bool {
        self.get_error().is_none()
    }

    fn id(&self) -> Option<&'_ str>;

    fn invalidate(&mut self) -> Result<bool, SessionError>;
}

pub trait Request: Headers + Io {
    type Read<'b>: Read<Error = Self::Error>
    where
        Self: 'b;

    type Attributes<'b>: DynStorage<'b, Error = Self::Error>
    where
        Self: 'b;

    type Session<'b>: Session
    where
        Self: 'b;

    fn query_string(&self) -> &'_ str;

    fn attrs(&self) -> Self::Attributes<'_>;

    fn session(&self) -> Self::Session<'_>;

    fn reader(&self) -> Self::Read<'_>;
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
    fn complete<R>(self, request: R) -> Result<Completion, Self::Error>
    where
        R: Request,
        Self: Sized;
}

pub trait Response<const B: usize = 64>: SendStatus + SendHeaders + Io {
    type Write: ResponseWrite<Error = Self::Error>;

    fn send_bytes<R>(self, request: R, bytes: &[u8]) -> Result<Completion, Self::Error>
    where
        R: Request,
        Self: Sized,
    {
        let mut write = self.into_writer()?;

        write.write_all(bytes.as_ref())?;

        write.complete(request)
    }

    fn send_str<R>(self, request: R, s: &str) -> Result<Completion, Self::Error>
    where
        R: Request,
        Self: Sized,
    {
        self.send_bytes(request, s.as_bytes())
    }

    #[cfg(feature = "alloc")]
    fn send_json<R, T>(
        self,
        request: R,
        o: &T,
    ) -> Result<Completion, EitherError<Self::Error, serde_json::Error>>
    where
        R: Request,
        T: serde::Serialize + ?Sized,
        Self: Sized,
    {
        let s = serde_json::to_string(o).map_err(EitherError::E2)?;

        self.send_str(request, &s).map_err(EitherError::E1)
    }

    fn send_reader<R, I>(
        self,
        request: R,
        size: Option<usize>,
        read: I,
    ) -> Result<Completion, EitherError<Self::Error, I::Error>>
    where
        R: Request,
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

        write.complete(request).map_err(EitherError::E1)
    }

    fn into_writer(self) -> Result<Self::Write, Self::Error>
    where
        Self: Sized;

    fn submit<R>(self, request: R) -> Result<Completion, Self::Error>
    where
        R: Request,
        Self: Sized,
    {
        self.send_bytes(request, &[0_u8; 0])
    }

    fn redirect<R>(self, request: R, location: &str) -> Result<Completion, Self::Error>
    where
        R: Request,
        Self: Sized,
    {
        self.header("location", location).submit(request)
    }
}

pub trait Context: Io {
    type Request: Request<Error = Self::Error>;
    type Response: Response<Error = Self::Error>;
}

#[cfg(feature = "alloc")]
mod response_data {
    extern crate alloc;
    use alloc::boxed::Box;
    use alloc::collections::BTreeMap;
    use alloc::string::String;
    use alloc::vec::Vec;

    use crate::http::{SendHeaders, SendStatus};
    use crate::io::{Error, ErrorKind, Io, Read};

    struct ErasedErrorRead<R>(R);

    impl<R> Io for ErasedErrorRead<R>
    where
        R: Io,
    {
        type Error = ErrorKind;
    }

    impl<R> Read for ErasedErrorRead<R>
    where
        R: Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.0.read(buf).map_err(|e| e.kind())
        }
    }

    pub enum Body {
        Empty,
        Bytes(Vec<u8>),
        Read(Option<usize>, Box<dyn Read<Error = ErrorKind>>),
    }

    impl Body {
        pub fn from_json<T: ?Sized + serde::Serialize>(
            data: &T,
        ) -> Result<Self, serde_json::Error> {
            Ok(serde_json::to_string(data)?.into())
        }

        pub fn is_empty(&self) -> bool {
            match self.len() {
                None => false,
                Some(len) => len == 0,
            }
        }

        pub fn len(&self) -> Option<usize> {
            match self {
                Body::Empty => Some(0),
                Body::Bytes(v) => Some(v.len()),
                Body::Read(Some(len), _) => Some(*len),
                _ => None,
            }
        }
    }

    impl Default for Body {
        fn default() -> Self {
            Body::Empty
        }
    }

    impl From<Vec<u8>> for Body {
        fn from(v: Vec<u8>) -> Self {
            Body::Bytes(v)
        }
    }

    impl From<String> for Body {
        fn from(s: String) -> Self {
            Body::Bytes(s.into())
        }
    }

    impl From<&str> for Body {
        fn from(s: &str) -> Self {
            Body::Bytes(s.into())
        }
    }

    #[cfg(feature = "std")]
    impl From<std::fs::File> for Body {
        fn from(f: std::fs::File) -> Self {
            Body::Read(
                f.metadata().map_or(None, |md| Some(md.len() as usize)),
                Box::new(ErasedErrorRead(crate::io::adapters::FromStd::new(f))),
            )
        }
    }

    pub struct ResponseData {
        pub(crate) status: u16,
        pub(crate) status_message: Option<String>,

        pub(crate) headers: BTreeMap<String, String>,

        pub(crate) body: Body,
    }

    impl Default for ResponseData {
        fn default() -> Self {
            ResponseData {
                status: 200,
                status_message: None,
                headers: BTreeMap::new(),
                body: Default::default(),
            }
        }
    }

    impl From<()> for ResponseData {
        fn from(_: ()) -> Self {
            Default::default()
        }
    }

    impl From<u16> for ResponseData {
        fn from(status: u16) -> Self {
            Self::new(status)
        }
    }

    impl From<Vec<u8>> for ResponseData {
        fn from(v: Vec<u8>) -> Self {
            Self::ok().body(v.into())
        }
    }

    impl From<&str> for ResponseData {
        fn from(s: &str) -> Self {
            Self::ok().body(s.into())
        }
    }

    impl From<String> for ResponseData {
        fn from(s: String) -> Self {
            Self::ok().body(s.into())
        }
    }

    #[cfg(feature = "std")]
    impl From<std::fs::File> for ResponseData {
        fn from(f: std::fs::File) -> Self {
            Self::ok().body(f.into())
        }
    }

    impl ResponseData {
        pub fn ok() -> Self {
            Default::default()
        }

        pub fn redirect(location: impl Into<String>) -> Self {
            Self::new(301).header("location", location.into())
        }

        pub fn new(status_code: u16) -> Self {
            Self::ok().status(status_code)
        }

        pub fn from_json<T: ?Sized + serde::Serialize>(
            data: &T,
        ) -> Result<Self, serde_json::Error> {
            Ok(Self::ok().body(Body::from_json(data)?))
        }

        pub fn from_err<E>(err: E) -> Self
        where
            E: core::fmt::Display + core::fmt::Debug,
        {
            Self::new(500)
                .status_message(format!("{}", err))
                .body(format!("{:#?}", err).into())
        }

        pub fn body(mut self, body: Body) -> Self {
            self.body = body;

            self
        }
    }

    impl SendStatus for ResponseData {
        fn set_status(&mut self, status: u16) -> &mut Self {
            self.status = status;
            self
        }

        fn set_status_message(&mut self, message: &str) -> &mut Self {
            self.status_message = Some(message.to_owned());
            self
        }
    }

    impl SendHeaders for ResponseData {
        fn set_header(&mut self, name: &str, value: &str) -> &mut Self {
            self.headers.insert(name.to_owned(), value.to_owned());
            self
        }
    }

    impl<E> From<ResponseData> for Result<ResponseData, E> {
        fn from(response: ResponseData) -> Self {
            Ok(response)
        }
    }
}
