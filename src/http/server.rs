use core::{any::Any, fmt};

extern crate alloc;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "std")]
use crate::io::Read;
use crate::io::{self, Write};
use crate::service::Service;

use super::{Headers, Method, SendHeaders, SendStatus};

pub mod attr;
pub mod middleware;
pub mod registry;
pub mod session;

pub trait Attributes<'a> {
    fn get(&self, name: impl AsRef<str>) -> Option<Rc<dyn Any>>;
    fn set(&mut self, name: impl AsRef<str>, value: Rc<dyn Any>) -> Option<Rc<dyn Any>>;
    fn remove(&mut self, name: impl AsRef<str>) -> Option<Rc<dyn Any>>;
}

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

pub trait Session<'a> {
    fn create_if_invalid(&mut self) -> Result<&mut Self, SessionError>;

    fn get_error(&self) -> Option<SessionError>;

    fn is_valid(&self) -> bool {
        self.get_error().is_none()
    }

    fn id(&self) -> Option<Cow<'_, str>>;

    fn get<T: DeserializeOwned>(&self, name: impl AsRef<str>) -> Result<Option<T>, SessionError>;
    fn set_and_get<S: Serialize, T: DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
        value: &S,
    ) -> Result<Option<T>, SessionError>;
    fn remove_and_get<T: DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
    ) -> Result<Option<T>, SessionError>;

    fn set<S: Serialize>(&mut self, name: impl AsRef<str>, value: &S)
        -> Result<bool, SessionError>;
    fn remove(&mut self, name: impl AsRef<str>) -> Result<bool, SessionError>;

    fn invalidate(&mut self) -> Result<bool, SessionError>;
}

pub trait Request<'a>: Headers + Service {
    type Read<'b>: io::Read<Error = Self::Error>
    where
        Self: 'b;

    type Attributes<'b>: Attributes<'b>
    where
        Self: 'b;

    type Session<'b>: Session<'b>
    where
        Self: 'b;

    fn query_string(&self) -> Cow<'_, str>;

    fn attrs(&self) -> Self::Attributes<'_>;

    fn session(&self) -> Self::Session<'_>;

    fn reader(&self) -> Self::Read<'_>;
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

struct PrivateData;

pub struct Completion(PrivateData);

impl Completion {
    /// # Safety
    /// This function is marked as unsafe because it is an internal API and is NOT supposed to be called by the user
    pub unsafe fn internal_new() -> Self {
        Self(PrivateData)
    }
}

pub trait ResponseWrite<'a>: io::Write {
    fn complete(self) -> Result<Completion, Self::Error>;
}

pub trait Response<'a>: SendStatus<'a> + SendHeaders<'a> + Service {
    type Write<'b>: ResponseWrite<'b, Error = Self::Error>;

    fn send_bytes(
        self,
        request: impl Request<'a>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        let mut write = self.into_writer(request)?;

        write.do_write_all(bytes.as_ref())?;

        write.complete()
    }

    fn send_str(
        self,
        request: impl Request<'a>,
        s: impl AsRef<str>,
    ) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(request, s.as_ref().as_bytes())
    }

    fn send_json<T: Serialize>(
        self,
        request: impl Request<'a>,
        o: impl AsRef<T>,
    ) -> Result<Completion, SendError<Self::Error, serde_json::Error>>
    where
        Self: Sized,
    {
        let s = serde_json::to_string(o.as_ref()).map_err(SendError::WriteError)?;

        self.send_str(request, s).map_err(SendError::SendError)
    }

    fn send_reader<R: io::Read>(
        self,
        request: impl Request<'a>,
        size: Option<usize>,
        read: R,
    ) -> Result<Completion, SendError<Self::Error, R::Error>>
    where
        Self: Sized,
    {
        let mut write = self.into_writer(request).map_err(SendError::SendError)?;

        if let Some(size) = size {
            io::copy_len(read, &mut write, size as u64)
        } else {
            io::copy(read, &mut write)
        }
        .map_err(|e| match e {
            io::CopyError::ReadError(e) => SendError::WriteError(e),
            io::CopyError::WriteError(e) => SendError::SendError(e),
        })?;

        write.complete().map_err(SendError::SendError)
    }

    fn into_writer(self, request: impl Request<'a>) -> Result<Self::Write<'a>, Self::Error>;

    fn submit(self, request: impl Request<'a>) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(request, &[0_u8; 0])
    }

    fn redirect(
        self,
        request: impl Request<'a>,
        location: impl Into<Cow<'a, str>>,
    ) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        self.header("location", location).submit(request)
    }
}

pub enum Body {
    Empty,
    Bytes(Vec<u8>),
    Read(Option<usize>, Box<dyn io::Read<Error = io::IODynError>>),
}

impl Body {
    pub fn from_json<T: ?Sized + serde::Serialize>(data: &T) -> Result<Self, serde_json::Error> {
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
            io::StdRead(f).into_dyn_read(),
        )
    }
}

pub struct ResponseData {
    status: u16,
    status_message: Option<String>,

    headers: BTreeMap<String, String>,

    body: Body,
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

    pub fn from_json<T: ?Sized + serde::Serialize>(data: &T) -> Result<Self, serde_json::Error> {
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

impl SendStatus<'static> for ResponseData {
    fn set_status(&mut self, status: u16) -> &mut Self {
        self.status = status;
        self
    }

    fn set_status_message<M>(&mut self, message: M) -> &mut Self
    where
        M: Into<Cow<'static, str>>,
    {
        self.status_message = Some(message.into().into_owned());
        self
    }
}

impl SendHeaders<'static> for ResponseData {
    fn set_header<H, V>(&mut self, name: H, value: V) -> &mut Self
    where
        H: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        self.headers
            .insert(name.into().into_owned(), value.into().into_owned());
        self
    }
}

impl<E> From<ResponseData> for Result<ResponseData, E> {
    fn from(response: ResponseData) -> Self {
        Ok(response)
    }
}
