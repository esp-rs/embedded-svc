use std::{any::Any, collections::HashMap, fs, io::{self, Read}, sync::{Arc, RwLock}};

pub use anyhow::Result;

pub enum Body {
    Empty,
    Bytes(Vec<u8>),
    Read(Option<usize>, Box<dyn io::Read>)
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
    fn default () -> Self {
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

impl From<fs::File> for Body {
    fn from(f: fs::File) -> Self {
        Body::Read(
            f.metadata().map_or(None, |md| Some(md.len() as usize)),
            Box::new(f))
    }
}

pub type StateMap = HashMap<String, Box<dyn Any>>;
pub type State = Arc<RwLock<StateMap>>;

pub trait RequestDelegate {
    fn header(&self, name: &str) -> Option<String>;
    fn url(&self) -> String;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error>;
}

pub struct Request {
    delegate: Box<dyn RequestDelegate>,
    attrs: StateMap,
    session: Option<State>,
    app: Option<State>
}

impl Request {
    pub fn new(
            delegate: Box<dyn RequestDelegate>,
            attribs: StateMap,
            session: Option<State>,
            app: Option<State>) -> Self {
        Request {
            delegate,
            attrs: attribs,
            session,
            app
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
            .map(|v| v
                    .as_str()
                    .parse::<usize>()
                    .map_or(None, Some))
                .flatten()
    }

    pub fn url(&self) -> String {
        self.delegate.url()
    }

    pub fn as_string(&mut self) -> io::Result<String> {
        let mut s = String::new();

        self.read_to_string(&mut s).map(|_| s)
    }

    pub fn as_bytes(&mut self) -> io::Result<Vec<u8>> {
        let mut v = vec! [];

        self.read_to_end(&mut v).map(|_| v)
    }

    pub fn attrs(&self) -> &StateMap {
        &self.attrs
    }

    pub fn attrs_mut(&mut self) -> &mut StateMap {
        &mut self.attrs
    }

    pub fn session(&self) -> Option<&State> {
        self.session.as_ref()
    }

    pub fn app(&self) -> &State {
        &self.app.as_ref().unwrap()
    }
}

impl io::Read for Request {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.delegate.read(buf)
    }
}

pub struct Response {
    pub status: u16,
    pub status_message: Option<String>,

    pub headers: HashMap<String, String>,

    pub body: Body,

    pub new_session: Option<StateMap>
}

impl Default for Response {
    fn default() -> Self {
        Response {
            status: 200,
            status_message: None,
            headers: HashMap::new(),
            body: Default::default(),
            new_session: None,
        }
    }
}

impl From<()> for Response {
    fn from(_: ()) -> Self {
        ResponseBuilder::ok().into()
    }
}

impl From<u16> for Response {
    fn from(status: u16) -> Self {
        ResponseBuilder::new(status).into()
    }
}

impl From<Vec<u8>> for Response {
    fn from(v: Vec<u8>) -> Self {
        ResponseBuilder::ok().body(v.into()).into()
    }
}

impl From<&str> for Response {
    fn from(s: &str) -> Self {
        ResponseBuilder::ok().body(s.into()).into()
    }
}

impl From<String> for Response {
    fn from(s: String) -> Self {
        ResponseBuilder::ok().body(s.into()).into()
    }
}

impl From<fs::File> for Response {
    fn from(f: fs::File) -> Self {
        ResponseBuilder::ok().body(f.into()).into()
    }
}

pub struct ResponseBuilder(Response);

impl ResponseBuilder {
    pub fn ok() -> Self {
        ResponseBuilder(Response {
            ..Default::default()
        })
    }

    pub fn new(status_code: u16) -> Self {
        ResponseBuilder::ok().status(status_code)
    }

    pub fn status(mut self, status: u16) -> Self {
        self.0.status = status;

        self
    }

    pub fn status_message(mut self, message: impl Into<String>) -> Self {
        self.0.status_message = Some(message.into());

        self
    }

    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.0.headers.insert(name.into(), value.into());

        self
    }

    pub fn content_type(self, value: impl Into<String>) -> Self {
        self.header("content-type", value.into())
    }

    pub fn content_len(self, value: usize) -> Self {
        self.header("content-length", value.to_string())
    }

    pub fn body(mut self, body: Body) -> Self {
        self.0.body = body;

        self
    }

    pub fn new_session(mut self, new_session: StateMap) -> Self {
        self.0.new_session = Some(new_session);

        self
    }
}

impl From<ResponseBuilder> for Response {
    fn from(builder: ResponseBuilder) -> Self {
        builder.0
    }
}

impl From<ResponseBuilder> for Result<Response> {
    fn from(builder: ResponseBuilder) -> Self {
        Ok(builder.0)
    }
}

impl From<anyhow::Error> for Response {
    fn from(err: anyhow::Error) -> Self {
        ResponseBuilder::new(500)
            .status_message(err.to_string())
            .body(format!("{:#}", err).into())
            .into()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
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
    handler: Box<dyn Fn(Request) -> Result<Response>>
}

pub struct Middleware {
    uri: String,
    handler: Box<dyn for <'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response>>
}

impl Middleware {
    pub fn new(
            uri: impl ToString,
            handler: impl for <'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> + 'static) -> Self {
        Middleware {
            uri: uri.to_string(),
            handler: Box::new(handler),
        }
    }

    pub fn uri(&self) -> &impl AsRef<str> {
        &self.uri
    }

    pub fn handler(self) -> Box<dyn for <'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response>> {
        self.handler
    }
}

impl Handler {
    pub fn new(uri: impl ToString, method: Method, handler: impl Fn(Request) -> Result<Response> + 'static) -> Self {
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
    use std::{sync::Arc, vec};
    use super::Result;

    use crate::httpd::{Request, Response, Handler, Middleware, Method};

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

    impl<RR> RegistryBuilder<RR> where RR: Registry {
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

        pub fn middleware(self, m: impl for <'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> + 'static) -> Result<RR> {
            self.registry.middleware(Middleware::new(self.uri, m))
        }

        pub fn handler(self, method: Method, f: impl Fn(Request) -> Result<Response> + 'static) -> Result<RR> {
            self.registry.handler(Handler::new(self.uri, method, f))
        }
    }

    #[derive(Default)]
    pub struct MiddlewareRegistry {
        handlers: Vec<Handler>,
        middlewares: Vec<Arc<Middleware>>
    }

    impl MiddlewareRegistry {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
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

        fn apply(middleware: Arc<Middleware>, handler: Box<dyn Fn(Request) -> Result<Response>>) -> Box<dyn Fn(Request) -> Result<Response>> {
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
    use std::sync::{Arc, RwLock};

    use super::{Request, Response, Result, State, StateMap};

    pub fn middleware(app: StateMap) -> impl for <'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
        let app = Arc::new(RwLock::new(app));

        move |request, handler| handle(&app, request, handler)
    }

    fn handle(app: &State, request: Request, handler: &dyn Fn(Request) -> Result<Response>) -> Result<Response> {
        handler(Request::new(request.delegate, request.attrs, request.session, Some(app.clone())))
    }
}

pub mod sessions {
    use std::{collections::HashMap, fmt::Write, sync::{Arc, Mutex, RwLock}};

    use super::{Request, Response, ResponseBuilder, State, Result};

    pub fn middleware(sessions: Sessions) -> impl for <'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
        let sessions = Mutex::new(sessions);

        move |request, handler| Sessions::handle(&sessions, request, handler)
    }

    pub struct Sessions {
        max_sessions: usize,
        data: HashMap<String, SessionData>
    }

    impl Default for Sessions {
        fn default() -> Self {
            Self {
                max_sessions: 16,
                data: HashMap::new()
            }
        }
    }

    impl Sessions {
        pub fn new(max_sessions: usize) -> Self {
            Self {
                max_sessions,
                ..Default::default()
            }
        }

        fn handle(sessions: &Mutex<Sessions>, request: Request, handler: &dyn Fn(Request) -> Result<Response>) -> Result<Response> {
            let session_id = Self::get_session_id(&request);

            let session = session_id
                .as_ref()
                .map(|s| sessions.lock().unwrap().get(s.as_str()))
                .flatten();

            let response = handler(Request::new(request.delegate, request.attrs, session, request.app))?;

            Ok(sessions.lock().unwrap().update(session_id.as_ref().map(String::as_str), response))
        }

        fn invalidate(&mut self, session_id: &str) -> bool {
            match self.data.remove(session_id) {
                Some(_) => true,
                None => false,
            }
        }

        fn get_session_id(req: &Request) -> Option<String> {
            req
                .header("Cookie")
                .map(|v| Self::parse_session_cookie(v.as_str()))
                .flatten()
        }

        fn get(&mut self, session_id: &str) -> Option<State> {
            if let Some(session_data) = self.data.get_mut(session_id) {
                let now = std::time::Instant::now();

                if session_data.used > 0 || session_data.last_accessed + session_data.session_timeout > now {
                    session_data.last_accessed = now;
                    session_data.used += 1;
                    Some(session_data.data.clone())
                } else {
                    self.data.remove(session_id);

                    None
                }
            } else {
                None
            }
        }

        fn update(&mut self, session_id: Option<&str>, mut resp: Response) -> Response {
            if let Some(new_session) = resp.new_session {
                let new_sess = session_id
                    .map_or(true, |s| self.data.remove(s).is_none());

                if new_sess && self.data.len() == self.max_sessions {
                    ResponseBuilder::new(429).into()
                } else {
                    let new_session_id = Self::generate_session_id();

                    resp.headers.insert("set-cookie".into(), Self::insert_session_cookie("", &new_session_id));
                    resp.new_session = None;

                    self.data.insert(new_session_id, SessionData {
                        last_accessed: std::time::Instant::now(),
                        session_timeout: std::time::Duration::from_secs(20 * 60),
                        used: 0,
                        data: Arc::new(RwLock::new(new_session))
                    });

                    resp
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
            let now = std::time::Instant::now();

            self.data.retain(|_, sd| sd.last_accessed + sd.session_timeout > now);
        }

        fn generate_session_id() -> String {
            let mut new_session_id_bytes = [0u8; 16];
            getrandom::getrandom(&mut new_session_id_bytes).unwrap();

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

            write!(&mut new_session_id, "{:x}", ByteBuf(&new_session_id_bytes)).expect("Unable to write");

            new_session_id
        }

        fn parse_session_cookie(_cookies: &str) -> Option<String> {
            Some("todo".into()) // TODO: Fetch from cookie header
        }

        fn insert_session_cookie(_cookies: &str, session_id: &str) -> String {
            // TODO: Fix the cookie handling code
            let mut cookie_str = String::new();
            write!(&mut cookie_str, "SESSIONID={}", session_id).unwrap();

            cookie_str
        }
    }

    struct SessionData {
        last_accessed: std::time::Instant,
        session_timeout: std::time::Duration,
        used: u32,
        data: State
    }
}
