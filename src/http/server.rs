use core::fmt;

extern crate alloc;
use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::io::{self, Read, Write};

use super::{Headers, Method, SendHeaders, SendStatus};

pub trait Request<'a>: Headers {
    type Read<'b>: io::Read<Error = Self::Error>
    where
        Self: 'b;

    #[cfg(not(feature = "std"))]
    type Error: fmt::Debug + fmt::Display;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn query_string(&self) -> Cow<'_, str>;

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
    pub unsafe fn internal_new() -> Self {
        Self(PrivateData)
    }
}

pub trait ResponseWrite<'a>: io::Write {
    fn complete(self) -> Result<Completion, Self::Error>;
}

pub trait InlineResponse<'a>: SendStatus<'a> + SendHeaders<'a> {
    type Write<'b>: ResponseWrite<'b, Error = Self::Error>;

    #[cfg(not(feature = "std"))]
    type Error: fmt::Debug + fmt::Display;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

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

    fn send_json<T>(
        self,
        _request: impl Request<'a>,
        _t: impl AsRef<T>,
    ) -> Result<Completion, Self::Error>
    where
        Self: Sized,
    {
        todo!()
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
            io::StdIO(f).into_dyn_read(),
        )
    }
}

pub struct Response {
    status: u16,
    status_message: Option<String>,

    headers: BTreeMap<String, String>,

    body: Body,
    //pub new_session_state: Option<SessionState>,
}

impl Default for Response {
    fn default() -> Self {
        Response {
            status: 200,
            status_message: None,
            headers: BTreeMap::new(),
            body: Default::default(),
            //new_session_state: None,
        }
    }
}

impl From<()> for Response {
    fn from(_: ()) -> Self {
        Default::default()
    }
}

impl From<u16> for Response {
    fn from(status: u16) -> Self {
        Self::new(status)
    }
}

impl From<Vec<u8>> for Response {
    fn from(v: Vec<u8>) -> Self {
        Self::ok().body(v.into())
    }
}

impl From<&str> for Response {
    fn from(s: &str) -> Self {
        Self::ok().body(s.into())
    }
}

impl From<String> for Response {
    fn from(s: String) -> Self {
        Self::ok().body(s.into())
    }
}

#[cfg(feature = "std")]
impl From<std::fs::File> for Response {
    fn from(f: std::fs::File) -> Self {
        Self::ok().body(f.into())
    }
}

impl Response {
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

    // pub fn new_session_state(mut self, new_session_state: SessionState) -> Self {
    //     self.new_session_state = Some(new_session_state);

    //     self
    // }
}

impl SendStatus<'static> for Response {
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

impl SendHeaders<'static> for Response {
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

impl<E> From<Response> for Result<Response, E> {
    fn from(response: Response) -> Self {
        Ok(response)
    }
}

pub struct HandlerRegistration<H> {
    uri: String,
    method: Method,
    handler: H,
}

impl<H> HandlerRegistration<H> {
    pub fn new(uri: impl Into<String>, method: Method, handler: H) -> Self {
        Self {
            uri: uri.into(),
            method,
            handler,
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn handler(self) -> H {
        self.handler
    }
}

pub trait Middleware<R>
where
    R: Registry,
{
    type Error: fmt::Display + fmt::Debug;

    fn handle<'a, H, E>(
        &self,
        req: R::Request<'a>,
        resp: R::InlineResponse<'a>,
        handler: &H,
    ) -> Result<Completion, Self::Error>
    where
        R: Registry,
        H: for<'b> Fn(R::Request<'b>, R::InlineResponse<'b>) -> Result<Completion, E>,
        E: fmt::Display + fmt::Debug;
}

// pub struct TestMiddleware;

// impl<R> Middleware<R> for TestMiddleware
// where
//     R: Registry,
// {
//     type Error = anyhow::Error;

//     fn handle<'a, H, E>(
//         &self,
//         req: R::Request<'a>,
//         resp: R::InlineResponse<'a>,
//         handler: &H,
//     ) -> Result<Completion, Self::Error>
//     where
//         R: Registry,
//         H: for<'b> Fn(R::Request<'b>, R::InlineResponse<'b>) -> Result<Completion, E>,
//         E: fmt::Display + fmt::Debug,
//     {
//         if req.header("foo").is_some() {
//             anyhow::bail!("Boo");
//         }

//         handler(req, resp).map_err(|e| anyhow::format_err!("{}", e))
//     }
// }

pub trait Registry: Sized {
    type Request<'a>: Request<'a>;
    type InlineResponse<'a>: InlineResponse<'a>;

    #[cfg(not(feature = "std"))]
    type Error: fmt::Debug + fmt::Display;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn set_inline_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::InlineResponse<'a>) -> Result<Completion, E>
            + 'static,
        E: fmt::Display + fmt::Debug;

    fn set_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        Self: 'static,
        H: for<'a, 'c> Fn(&'c mut Self::Request<'a>) -> Result<Response, E> + 'static + Clone,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<Self as Registry>::InlineResponse<'a> as InlineResponse<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.set_inline_handler(uri, method, into_boxed_inline_handler(handler))
    }

    fn with_middleware<M>(&mut self, middleware: M) -> MiddlewareRegistry<'_, Self, M>
    where
        M: Middleware<Self>,
    {
        MiddlewareRegistry {
            registry: self,
            middleware,
        }
    }

    fn at(&mut self, uri: impl ToString) -> HandlerRegistrationBuilder<Self> {
        HandlerRegistrationBuilder {
            uri: uri.to_string(),
            registry: self,
        }
    }

    fn register<R: FnOnce(Self) -> Result<Self, Self::Error>>(
        self,
        register: R,
    ) -> Result<Self, Self::Error> {
        register(self)
    }
}

pub struct MiddlewareRegistry<'r, RR, M> {
    registry: &'r mut RR,
    middleware: M,
}

impl<'r, R, M> Registry for MiddlewareRegistry<'r, R, M>
where
    R: Registry + 'static,
    M: Middleware<R> + Clone + 'static,
    M::Error: 'static,
{
    type Request<'a> = R::Request<'a>;
    type InlineResponse<'a> = R::InlineResponse<'a>;

    type Error = R::Error;

    fn set_inline_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::InlineResponse<'a>) -> Result<Completion, E>
            + 'static,
        E: fmt::Debug + fmt::Display,
    {
        let middleware = self.middleware.clone();

        self.registry
            .set_inline_handler(uri, method, move |req, resp| {
                middleware.handle(req, resp, &handler)
            })?;

        Ok(self)
    }
}

pub struct InlineHandlerRegistrationBuilder<'r, RR> {
    uri: String,
    registry: &'r mut RR,
}

impl<'r, RR> InlineHandlerRegistrationBuilder<'r, RR>
where
    RR: Registry,
{
    pub fn get<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a> Fn(RR::Request<'a>, RR::InlineResponse<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Get, handler)
    }

    pub fn post<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a> Fn(RR::Request<'a>, RR::InlineResponse<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Post, handler)
    }

    pub fn handler<H, E>(self, method: Method, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a> Fn(RR::Request<'a>, RR::InlineResponse<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.registry
            .set_inline_handler(self.uri.as_str(), method, handler)
    }
}

pub struct HandlerRegistrationBuilder<'r, RR> {
    uri: String,
    registry: &'r mut RR,
}

impl<'r, RR> HandlerRegistrationBuilder<'r, RR>
where
    RR: Registry + 'static,
{
    pub fn inline(self) -> InlineHandlerRegistrationBuilder<'r, RR> {
        InlineHandlerRegistrationBuilder {
            uri: self.uri,
            registry: self.registry,
        }
    }

    pub fn get<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<Response, E> + 'static + Clone,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<RR as Registry>::InlineResponse<'a> as InlineResponse<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Get, handler)
    }

    pub fn post<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<Response, E> + 'static + Clone,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<RR as Registry>::InlineResponse<'a> as InlineResponse<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Post, handler)
    }

    pub fn handler<H, E>(self, method: Method, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<Response, E> + 'static + Clone,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<RR as Registry>::InlineResponse<'a> as InlineResponse<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.registry
            .set_handler(self.uri.as_str(), method, handler)
    }
}

fn into_boxed_inline_handler<RR, H, E>(
    handler: H,
) -> Box<dyn for<'a> Fn(RR::Request<'a>, RR::InlineResponse<'a>) -> Result<Completion, E>>
where
    RR: Registry,
    H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<Response, E> + 'static + Clone,
    E: fmt::Debug
        + fmt::Display
        + for<'a> From<<<RR as Registry>::InlineResponse<'a> as InlineResponse<'a>>::Error>
        + From<io::IODynError>,
{
    Box::new(move |req, resp| handle::<RR, _, _>(req, resp, &handler))
}

fn handle<'b, RR, H, E>(
    mut req: RR::Request<'b>,
    mut inline_resp: RR::InlineResponse<'b>,
    handler: &H,
) -> Result<Completion, E>
where
    RR: Registry,
    H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<Response, E>,
    E: fmt::Debug
        + fmt::Display
        + From<<<RR as Registry>::InlineResponse<'b> as InlineResponse<'b>>::Error>
        + From<io::IODynError>,
{
    let resp = handler(&mut req)?;

    inline_resp.set_status(resp.status);

    if let Some(status_message) = resp.status_message {
        inline_resp.set_status_message(status_message);
    }

    for (key, value) in resp.headers {
        inline_resp.set_header(key, value);
    }

    match resp.body {
        Body::Empty => inline_resp.submit(req).map_err(Into::into),
        Body::Bytes(bytes) => inline_resp.send_bytes(req, &bytes).map_err(Into::into),
        Body::Read(size, reader) => {
            inline_resp
                .send_reader(req, size, reader)
                .map_err(|e| match e {
                    SendError::SendError(e) => e.into(),
                    SendError::WriteError(e) => e.into(),
                })
        }
    }
}

// fn test<'a>(req: &mut impl Request<'a>) -> Result<Response, anyhow::Error> {
//     let h1 = req.header("test").unwrap();

//     let mut xxx = [0_u8; 512];
//     let mut reader = req.reader();
//     reader.do_read(&mut xxx)?;

//     let mut v: Vec<u8> = Vec::new();
//     io::StdIO(reader).read_to_end(&mut v)?;

//     Response::ok().status_message(h1.into_owned()).into()
// }
