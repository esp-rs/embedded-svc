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

pub trait InlineHandler<'a, REQ: Request<'a>, RESP: InlineResponse<'a>> {
    type Error: core::fmt::Debug + core::fmt::Display;

    fn handle(&self, req: REQ, resp: RESP) -> Result<Completion, Self::Error>;
}

impl<'a, F, REQ, RESP, E> InlineHandler<'a, REQ, RESP> for F
where
    F: Fn(REQ, RESP) -> Result<Completion, E>,
    REQ: Request<'a>,
    RESP: InlineResponse<'a>,
    E: core::fmt::Debug + core::fmt::Display,
{
    type Error = E;

    fn handle(&self, req: REQ, resp: RESP) -> Result<Completion, Self::Error> {
        (self)(req, resp)
    }
}

pub trait Handler<'a, R: Request<'a>> {
    type Error: fmt::Debug + fmt::Display;

    fn handle(&self, req: &mut R) -> Result<Response, Self::Error>;
}

impl<'a, F, R, E> Handler<'a, R> for F
where
    F: Fn(&mut R) -> Result<Response, E>,
    R: Request<'a>,
    E: fmt::Debug + fmt::Display,
{
    type Error = E;

    fn handle(&self, req: &mut R) -> Result<Response, Self::Error> {
        (self)(req)
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

pub trait Registry: Sized {
    type Request<'a>: Request<'a>;
    type Response<'a>: InlineResponse<'a>;

    #[cfg(not(feature = "std"))]
    type Error: fmt::Debug + fmt::Display;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn set_inline_handler<'a, F>(
        &mut self,
        handler: HandlerRegistration<F>,
    ) -> Result<&mut Self, Self::Error>
    where
        F: InlineHandler<'a, Self::Request<'a>, Self::Response<'a>>;

    fn at(self, uri: impl ToString) -> HandlerRegistrationBuilder<Self> {
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

pub struct HandlerRegistrationBuilder<RR> {
    uri: String,
    registry: RR,
}

impl<RR> HandlerRegistrationBuilder<RR>
where
    RR: Registry,
{
    pub fn inline(self) -> InlineHandlerRegistrationBuilder<RR> {
        InlineHandlerRegistrationBuilder {
            uri: self.uri,
            registry: self.registry,
        }
    }

    pub fn get<'a, F>(self, f: F) -> Result<RR, RR::Error>
    where
        F: Handler<'a, RR::Request<'a>>,
        F::Error: From<
            SendError<
                <<RR as Registry>::Response<'a> as InlineResponse<'a>>::Error,
                std::io::Error,
            >,
        >,
    {
        self.handler(Method::Get, f)
    }

    pub fn post<'a, F>(self, f: F) -> Result<RR, RR::Error>
    where
        F: Handler<'a, RR::Request<'a>>,
        F::Error: From<
            SendError<
                <<RR as Registry>::Response<'a> as InlineResponse<'a>>::Error,
                std::io::Error,
            >,
        >,
    {
        self.handler(Method::Post, f)
    }

    pub fn handler<'a, F>(mut self, method: Method, f: F) -> Result<RR, RR::Error>
    where
        F: Handler<'a, RR::Request<'a>>,
        F::Error: From<
            SendError<
                <<RR as Registry>::Response<'a> as InlineResponse<'a>>::Error,
                std::io::Error,
            >,
        >,
    {
        self.registry.set_inline_handler(HandlerRegistration::new(
            self.uri,
            method,
            into_inline_handler(f),
        ))?;

        Ok(self.registry)
    }
}

pub struct InlineHandlerRegistrationBuilder<RR> {
    uri: String,
    registry: RR,
}

impl<RR> InlineHandlerRegistrationBuilder<RR>
where
    RR: Registry,
{
    pub fn get<'a, F>(self, f: F) -> Result<RR, RR::Error>
    where
        F: InlineHandler<'a, RR::Request<'a>, RR::Response<'a>>,
    {
        self.handler(Method::Get, f)
    }

    pub fn post<'a, F>(self, f: F) -> Result<RR, RR::Error>
    where
        F: InlineHandler<'a, RR::Request<'a>, RR::Response<'a>>,
    {
        self.handler(Method::Post, f)
    }

    pub fn handler<'a, F>(mut self, method: Method, f: F) -> Result<RR, RR::Error>
    where
        F: InlineHandler<'a, RR::Request<'a>, RR::Response<'a>>,
    {
        self.registry
            .set_inline_handler(HandlerRegistration::new(self.uri, method, f))?;

        Ok(self.registry)
    }
}

// fn test<'a>(req: &mut impl Request<'a>) -> Result<Response, anyhow::Error> {
//     let h1 = req.header("test").unwrap();

//     let mut xxx = [0_u8; 512];
//     req.do_read(&mut xxx)?;

//     let mut v: Vec<u8> = Vec::new();
//     io::StdIO(req).read_to_end(&mut v)?;

//     Response::ok().status_message(h1.into_owned()).into()
// }

pub fn into_inline_handler<'a, H, REQ, RESP>(h: H) -> impl InlineHandler<'a, REQ, RESP>
where
    H: Handler<'a, REQ>,
    REQ: Request<'a>,
    RESP: InlineResponse<'a>,
    H::Error: From<SendError<RESP::Error, std::io::Error>>,
{
    move |req, resp| handle(&h, req, resp)
}

fn handle<'a, H, REQ, RESP>(
    h: &H,
    mut req: REQ,
    mut inline_resp: RESP,
) -> Result<Completion, H::Error>
where
    H: Handler<'a, REQ>,
    REQ: Request<'a>,
    RESP: InlineResponse<'a>,
    H::Error: From<SendError<RESP::Error, std::io::Error>>,
{
    let resp = h.handle(&mut req)?;

    inline_resp.set_status(resp.status);

    if let Some(status_message) = resp.status_message {
        inline_resp.set_status_message(status_message);
    }

    for (key, value) in resp.headers {
        inline_resp.set_header(key, value);
    }

    match resp.body {
        Body::Empty => inline_resp.submit(req).map_err(|e| SendError::SendError(e)),
        Body::Bytes(bytes) => inline_resp
            .send_bytes(req, &bytes)
            .map_err(|e| SendError::SendError(e)),
        Body::Read(size, reader) => inline_resp.send_reader(req, size, reader),
    }
    .map_err(|e| e.into())
}
