use core::any::Any;

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

// TODO: Think how to model this in no_std without pushing generic params everywhere
use std::sync::RwLock;

// TODO: Perhaps replace these with the core2 polyfills
use std::io;
use std::io::Read;

#[cfg(feature = "std")]
use std::fs;

use enumset::*;

pub use anyhow::Error;
pub use anyhow::Result;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "use_strum")]
use strum_macros::{Display, EnumIter, EnumMessage, EnumString};

#[cfg(feature = "use_numenum")]
use num_enum::TryFromPrimitive;

pub enum Body {
    Empty,
    Bytes(Vec<u8>),
    #[cfg(feature = "std")]
    Read(Option<usize>, Box<dyn io::Read>),
}

impl Body {
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
impl From<fs::File> for Body {
    fn from(f: fs::File) -> Self {
        Body::Read(
            f.metadata().map_or(None, |md| Some(md.len() as usize)),
            Box::new(f),
        )
    }
}

pub type StateMap = BTreeMap<String, Box<dyn Any>>;

#[cfg(feature = "std")]
pub type State = Arc<RwLock<StateMap>>;

pub trait RequestDelegate {
    fn header(&self, name: &str) -> Option<String>;
    fn query_string(&self) -> Option<String>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error>;
}

pub struct Request {
    delegate: Box<dyn RequestDelegate>,
    attrs: StateMap,

    #[cfg(feature = "std")]
    session: Option<State>,

    #[cfg(feature = "std")]
    app: Option<State>,
}

impl Request {
    pub fn new(
        delegate: Box<dyn RequestDelegate>,
        attribs: StateMap,
        #[cfg(feature = "std")] session: Option<State>,
        #[cfg(feature = "std")] app: Option<State>,
    ) -> Self {
        Self {
            delegate,
            attrs: attribs,
            #[cfg(feature = "std")]
            session,
            #[cfg(feature = "std")]
            app,
        }
    }

    pub fn header(&self, name: impl AsRef<str>) -> Option<String> {
        self.delegate.header(name.as_ref())
    }

    pub fn content_type(&self) -> Option<String> {
        self.header("content-type")
    }

    pub fn content_len(&self) -> Option<usize> {
        self.header("content-length")
            .and_then(|v| v.as_str().parse::<usize>().ok())
    }

    pub fn query_string(&self) -> Option<String> {
        self.delegate.query_string()
    }

    pub fn as_string(&mut self) -> Result<String> {
        let mut s = String::new();

        Ok(self.read_to_string(&mut s).map(|_| s)?)
    }

    pub fn as_bytes(&mut self) -> Result<Vec<u8>> {
        let mut v = vec![];

        Ok(self.read_to_end(&mut v).map(|_| v)?)
    }

    pub fn attrs(&self) -> &StateMap {
        &self.attrs
    }

    pub fn attrs_mut(&mut self) -> &mut StateMap {
        &mut self.attrs
    }

    #[cfg(feature = "std")]
    pub fn session(&self) -> Option<&State> {
        self.session.as_ref()
    }

    #[cfg(feature = "std")]
    pub fn app(&self) -> &State {
        self.app.as_ref().unwrap()
    }
}

#[cfg(feature = "std")]
impl io::Read for Request {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.delegate.read(buf)
    }
}

pub enum SessionState {
    New(StateMap),
    Invalidate,
}

pub struct Response {
    pub status: u16,
    pub status_message: Option<String>,

    pub headers: BTreeMap<String, String>,

    pub body: Body,

    pub new_session_state: Option<SessionState>,
}

impl Default for Response {
    fn default() -> Self {
        Response {
            status: 200,
            status_message: None,
            headers: BTreeMap::new(),
            body: Default::default(),
            new_session_state: None,
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
impl From<fs::File> for Response {
    fn from(f: fs::File) -> Self {
        Self::ok().body(f.into())
    }
}

impl Response {
    pub fn ok() -> Self {
        Default::default()
    }

    pub fn redirect(location: impl Into<String>) -> Self {
        Self::new(301).header("location", location)
    }

    pub fn new(status_code: u16) -> Self {
        Self::ok().status(status_code)
    }

    #[must_use]
    pub fn status(mut self, status: u16) -> Self {
        self.status = status;

        self
    }

    #[must_use]
    pub fn status_message(mut self, message: impl Into<String>) -> Self {
        self.status_message = Some(message.into());

        self
    }

    #[must_use]
    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());

        self
    }

    #[must_use]
    pub fn content_encoding(self, value: impl Into<String>) -> Self {
        self.header("content-encoding", value.into())
    }

    #[must_use]
    pub fn content_type(self, value: impl Into<String>) -> Self {
        self.header("content-type", value.into())
    }

    #[must_use]
    pub fn content_len(self, value: usize) -> Self {
        self.header("content-length", value.to_string())
    }

    #[must_use]
    pub fn body(mut self, body: Body) -> Self {
        self.body = body;

        self
    }

    #[must_use]
    pub fn new_session_state(mut self, new_session_state: SessionState) -> Self {
        self.new_session_state = Some(new_session_state);

        self
    }
}

impl From<Response> for Result<Response> {
    fn from(response: Response) -> Self {
        Ok(response)
    }
}

impl From<anyhow::Error> for Response {
    fn from(err: anyhow::Error) -> Self {
        Response::new(500)
            .status_message(err.to_string())
            .body(format!("{:#?}", err).into())
    }
}

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
pub enum Method {
    Delete,
    Get,
    Head,
    Post,
    Put,
    Connect,
    Options,
    Trace,
    Copy,
    Lock,
    MkCol,
    Move,
    Propfind,
    Proppatch,
    Search,
    Unlock,
    Bind,
    Rebind,
    Unbind,
    Acl,
    Report,
    MkActivity,
    Checkout,
    Merge,
    MSearch,
    Notify,
    Subscribe,
    Unsubscribe,
    Patch,
    Purge,
    MkCalendar,
    Link,
    Unlink,
}

pub struct Handler {
    uri: String,
    method: Method,
    handler: Box<dyn Fn(Request) -> Result<Response>>,
}

pub type BoxedMiddlewareHandler =
    Box<dyn for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response>>;

pub struct Middleware {
    uri: String,
    handler: BoxedMiddlewareHandler,
}

impl Middleware {
    pub fn new(
        uri: impl ToString,
        handler: impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response>
            + 'static,
    ) -> Self {
        Middleware {
            uri: uri.to_string(),
            handler: Box::new(handler),
        }
    }

    pub fn uri(&self) -> &impl AsRef<str> {
        &self.uri
    }

    #[allow(clippy::type_complexity)]
    pub fn handler(
        self,
    ) -> Box<dyn for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response>>
    {
        self.handler
    }
}

impl Handler {
    pub fn new(
        uri: impl ToString,
        method: Method,
        handler: impl Fn(Request) -> Result<Response> + 'static,
    ) -> Self {
        Handler {
            uri: uri.to_string(),
            method,
            handler: Box::new(handler),
        }
    }

    pub fn uri(&self) -> &impl AsRef<str> {
        &self.uri
    }

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn handler(self) -> Box<dyn Fn(Request) -> Result<Response>> {
        self.handler
    }
}

pub mod registry {
    extern crate alloc;

    use alloc::{string::*, sync::Arc, vec};

    use super::Result;

    use crate::httpd::{Handler, Method, Middleware, Request, Response};

    pub trait Registry: Sized {
        fn handler(self, handler: Handler) -> Result<Self>;
        fn middleware(self, middleware: Middleware) -> Result<Self>;

        fn at(self, uri: impl ToString) -> RegistryBuilder<Self> {
            RegistryBuilder {
                uri: uri.to_string(),
                registry: self,
            }
        }

        fn register<R: FnOnce(Self) -> Result<Self>>(self, register: R) -> Result<Self> {
            register(self)
        }
    }

    pub struct RegistryBuilder<RR> {
        uri: String,
        registry: RR,
    }

    impl<RR> RegistryBuilder<RR>
    where
        RR: Registry,
    {
        pub fn get(self, f: impl Fn(Request) -> Result<Response> + 'static) -> Result<RR> {
            self.handler(Method::Get, f)
        }

        pub fn post(self, f: impl Fn(Request) -> Result<Response> + 'static) -> Result<RR> {
            self.handler(Method::Post, f)
        }

        pub fn put(self, f: impl Fn(Request) -> Result<Response> + 'static) -> Result<RR> {
            self.handler(Method::Put, f)
        }

        pub fn delete(self, f: impl Fn(Request) -> Result<Response> + 'static) -> Result<RR> {
            self.handler(Method::Delete, f)
        }

        pub fn head(self, f: impl Fn(Request) -> Result<Response> + 'static) -> Result<RR> {
            self.handler(Method::Head, f)
        }

        pub fn middleware(
            self,
            m: impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response>
                + 'static,
        ) -> Result<RR> {
            self.registry.middleware(Middleware::new(self.uri, m))
        }

        pub fn handler(
            self,
            method: Method,
            f: impl Fn(Request) -> Result<Response> + 'static,
        ) -> Result<RR> {
            self.registry.handler(Handler::new(self.uri, method, f))
        }
    }

    #[derive(Default)]
    pub struct MiddlewareRegistry {
        handlers: Vec<Handler>,
        middlewares: Vec<Arc<Middleware>>,
    }

    impl MiddlewareRegistry {
        pub fn new() -> Self {
            Default::default()
        }

        pub fn apply_middleware(self) -> Vec<Handler> {
            let mut handlers: Vec<Handler> = vec![];

            for handler in self.handlers {
                let uri = handler.uri;
                let method = handler.method;
                let mut handler = handler.handler;

                for middleware in &self.middlewares {
                    handler = Self::apply(middleware.clone(), handler);
                }

                handlers.push(Handler::new(uri, method, handler));
            }

            handlers
        }

        fn apply(
            middleware: Arc<Middleware>,
            handler: Box<dyn Fn(Request) -> Result<Response>>,
        ) -> Box<dyn Fn(Request) -> Result<Response>> {
            Box::new(move |request| (middleware.handler)(request, &*handler))
        }
    }

    impl Registry for MiddlewareRegistry {
        fn handler(mut self, handler: Handler) -> Result<Self> {
            self.handlers.push(handler);
            Ok(self)
        }

        fn middleware(mut self, middleware: Middleware) -> Result<Self> {
            self.middlewares.push(Arc::new(middleware));
            Ok(self)
        }
    }
}

pub mod app {
    extern crate alloc;
    use alloc::sync::Arc;

    use std::sync::RwLock;

    use super::{Request, Response, Result, State, StateMap};

    pub fn middleware(
        app: StateMap,
    ) -> impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
        let app = Arc::new(RwLock::new(app));

        move |request, handler| handle(&app, request, handler)
    }

    fn handle(
        app: &State,
        request: Request,
        handler: &dyn Fn(Request) -> Result<Response>,
    ) -> Result<Response> {
        handler(Request::new(
            request.delegate,
            request.attrs,
            request.session,
            Some(app.clone()),
        ))
    }
}

pub mod sessions {
    use core::fmt::Write;

    extern crate alloc;
    use alloc::collections::BTreeMap;
    use alloc::sync::Arc;

    use std::sync::{Mutex, RwLock};

    use log::{info, warn};

    use super::{Request, Response, Result, SessionState, State};

    pub fn middleware<F: Fn() -> [u8; 16]>(
        sessions: Sessions<F>,
    ) -> impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
        let sessions = Mutex::new(sessions);

        move |request, handler| Sessions::handle(&sessions, request, handler)
    }

    pub struct Sessions<F> {
        max_sessions: usize,
        data: BTreeMap<String, SessionData>,
        get_random: F,
    }

    impl<F: Fn() -> [u8; 16]> Sessions<F> {
        pub fn new(max_sessions: usize, get_random: F) -> Self {
            Self {
                max_sessions,
                get_random,
                data: BTreeMap::new(),
            }
        }

        fn handle(
            sessions: &Mutex<Sessions<F>>,
            request: Request,
            handler: &dyn Fn(Request) -> Result<Response>,
        ) -> Result<Response> {
            let session_id = Self::get_session_id(&request);

            let session = session_id
                .as_ref()
                .and_then(|s| sessions.lock().unwrap().get(s.as_str()));

            let response = handler(Request::new(
                request.delegate,
                request.attrs,
                session,
                request.app,
            ))?;

            Ok(sessions
                .lock()
                .unwrap()
                .update(session_id.as_deref(), response))
        }

        fn invalidate(&mut self, session_id: &str) -> bool {
            info!("Invalidating session {}", session_id);

            self.data.remove(session_id).is_some()
        }

        fn get_session_id(req: &Request) -> Option<String> {
            req.header("cookie")
                .and_then(|v| Self::parse_session_cookie(v.as_str()))
        }

        fn get(&mut self, session_id: &str) -> Option<State> {
            if let Some(session_data) = self.data.get_mut(session_id) {
                let now = std::time::Instant::now();

                if session_data.used > 0
                    || session_data.last_accessed + session_data.session_timeout > now
                {
                    session_data.last_accessed = now;
                    session_data.used += 1;
                    Some(session_data.data.clone())
                } else {
                    self.invalidate(session_id);

                    None
                }
            } else {
                None
            }
        }

        fn update(&mut self, session_id: Option<&str>, mut resp: Response) -> Response {
            if let Some(new_session_state) = resp.new_session_state {
                match new_session_state {
                    SessionState::Invalidate => {
                        if let Some(session_id) = session_id {
                            self.invalidate(session_id);
                        }

                        resp.new_session_state = None;
                        resp
                    }
                    SessionState::New(new_session) => {
                        let new_sess = session_id.map_or(true, |s| self.data.remove(s).is_none());

                        if new_sess {
                            self.cleanup();
                        }

                        if new_sess && self.data.len() == self.max_sessions {
                            warn!(
                                "Cannot create a new session - max session limit ({}) exceeded",
                                self.max_sessions
                            );
                            Response::new(429)
                        } else {
                            let new_session_id = self.generate_session_id();

                            resp.headers.insert(
                                "set-cookie".into(),
                                Self::insert_session_cookie("", &new_session_id),
                            );

                            info!("New session {} created", &new_session_id);

                            self.data.insert(
                                new_session_id,
                                SessionData {
                                    last_accessed: std::time::Instant::now(),
                                    session_timeout: std::time::Duration::from_secs(20 * 60),
                                    used: 0,
                                    data: Arc::new(RwLock::new(new_session)),
                                },
                            );

                            resp.new_session_state = None;
                            resp
                        }
                    }
                }
            } else {
                if let Some(session_id) = session_id {
                    if let Some(session_data) = self.data.get_mut(session_id) {
                        session_data.last_accessed = std::time::Instant::now();
                        session_data.used -= 1;
                    }
                }

                resp
            }
        }

        fn cleanup(&mut self) {
            info!("Performing sessions cleanup");

            let now = std::time::Instant::now();

            self.data
                .retain(|_, sd| sd.last_accessed + sd.session_timeout > now);
        }

        fn generate_session_id(&self) -> String {
            let new_session_id_bytes = (self.get_random)();

            let mut new_session_id = String::new();

            struct ByteBuf<'a>(&'a [u8]);

            impl<'a> std::fmt::LowerHex for ByteBuf<'a> {
                fn fmt(&self, fmtr: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                    for byte in self.0 {
                        fmtr.write_fmt(format_args!("{:02x}", byte))?;
                    }

                    Ok(())
                }
            }

            write!(&mut new_session_id, "{:x}", ByteBuf(&new_session_id_bytes))
                .expect("Unable to write");

            new_session_id
        }

        fn parse_session_cookie(cookies: &str) -> Option<String> {
            for cookie in cookies.split(';') {
                let mut cookie_pair = cookie.split('=');

                if let Some(name) = cookie_pair.next() {
                    if name == "SESSIONID" {
                        if let Some(value) = cookie_pair.next() {
                            return Some(value.to_owned());
                        }
                    }
                }
            }

            None
        }

        fn insert_session_cookie(_cookies: &str, session_id: &str) -> String {
            let mut cookie_str = String::new();
            write!(&mut cookie_str, "SESSIONID={}", session_id).unwrap();

            cookie_str
        }
    }

    struct SessionData {
        last_accessed: std::time::Instant,
        session_timeout: std::time::Duration,
        used: u32,
        data: State,
    }
}
