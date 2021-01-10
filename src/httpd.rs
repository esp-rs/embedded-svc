use std::{cell::RefCell, collections::HashMap, fs, io};

pub enum Body {
    Empty,
    Bytes(Vec<u8>),
    Read(Option<usize>, Box<RefCell<dyn io::Read>>)
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
            Box::new(RefCell::new(f)))
    }
}

pub trait Request<S = (), A = ()>: io::Read {
    fn header(&self, name: impl AsRef<str>) -> Option<String>;

    fn content_type(&self) -> Option<String> {
        self.header("content-type")
    }

    fn content_len(&self) -> Option<usize> {
        self.header("content-length")
            .map(|v| v
                    .as_str()
                    .parse::<usize>()
                    .map_or(None, Some))
                .flatten()
    }

    fn url(&self) -> String;

    fn with_session<Q>(&self, f: impl FnOnce(Option<&S>) -> Q) -> Q;

    fn with_session_mut<Q>(&self, f: impl FnOnce(Option<&mut S>) -> Q) -> Q;

    fn with_app<Q>(&self, f: impl FnOnce(&A) -> Q) -> Q;

    fn with_app_mut<Q>(&self, f: impl FnOnce(&mut A) -> Q) -> Q;

    fn as_string(&mut self) -> io::Result<String> {
        let mut s = String::new();

        self.read_to_string(&mut s).map(|_| s)
    }

    fn as_bytes(&mut self) -> io::Result<Vec<u8>> {
        let mut v = vec! [];

        self.read_to_end(&mut v).map(|_| v)
    }
}

pub struct Response<S = ()> {
    pub status: u16,
    pub status_message: Option<String>,

    pub headers: HashMap<String, String>,

    pub body: Body,

    pub new_session: Option<S>,
}

impl<S> Default for Response<S> {
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

impl<S> From<()> for Response<S> {
    fn from(_: ()) -> Self {
        ResponseBuilder::ok().into()
    }
}

impl<S> From<u16> for Response<S> {
    fn from(status: u16) -> Self {
        ResponseBuilder::new(status).into()
    }
}

impl<S> From<Vec<u8>> for Response<S> {
    fn from(v: Vec<u8>) -> Self {
        ResponseBuilder::ok().body(v.into()).into()
    }
}

impl<S> From<&str> for Response<S> {
    fn from(s: &str) -> Self {
        ResponseBuilder::ok().body(s.into()).into()
    }
}

impl<S> From<String> for Response<S> {
    fn from(s: String) -> Self {
        ResponseBuilder::ok().body(s.into()).into()
    }
}

impl<S> From<fs::File> for Response<S> {
    fn from(f: fs::File) -> Self {
        ResponseBuilder::ok().body(f.into()).into()
    }
}

pub struct ResponseBuilder<S = ()>(Response<S>);

impl<S> ResponseBuilder<S> {
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

    pub fn status_message(mut self, message: String) -> Self {
        self.0.status_message = Some(message);
        
        self
    }

    pub fn header(mut self, name: &str, value: String) -> Self {
        self.0.headers.insert(name.into(), value);

        self
    }

    pub fn content_type(self, value: String) -> Self {
        self.header("content-type", value)
    }

    pub fn content_len(self, value: usize) -> Self {
        self.header("content-length", value.to_string())
    }

    pub fn body(mut self, body: Body) -> Self {
        self.0.body = body;

        self
    }

    pub fn new_session(mut self, new_session: S) -> Self {
        self.0.new_session = Some(new_session);

        self
    }
}

impl<S> From<ResponseBuilder<S>> for Response<S> {
    fn from(builder: ResponseBuilder<S>) -> Self {
        builder.0
    }
}

impl<S> From<anyhow::Error> for Response<S> {
    fn from(err: anyhow::Error) -> Self {
        ResponseBuilder::new(500)
            .status_message(err.to_string())
            .into()
    }
}

#[derive(Copy, Clone, PartialEq)]
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

pub struct Registration<R, S> {
    uri: String,
    method: Method,
    handler: Box<dyn Fn(&mut R) -> anyhow::Result<Response<S>>>,
}

impl<R, S> Registration<R, S> {
    pub fn new<A>(uri: impl ToString, method: Method, handler: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> Self where R: Request<S, A> {
        Registration {
            uri: uri.to_string(),
            method,
            handler: Box::new(handler),
        }
    }

    pub fn new_get<A>(uri: impl ToString, handler: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> Self where R: Request<S, A> {
        Registration::new(uri, Method::Get, handler)
    }

    pub fn new_post<A>(uri: impl ToString, handler: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> Self where R: Request<S, A> {
        Registration::new(uri, Method::Post, handler)
    }

    pub fn new_put<A>(uri: impl ToString, handler: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> Self where R: Request<S, A> {
        Registration::new(uri, Method::Put, handler)
    }

    pub fn new_delete<A>(uri: impl ToString, handler: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> Self where R: Request<S, A> {
        Registration::new(uri, Method::Delete, handler)
    }

    pub fn new_head<A>(uri: impl ToString, handler: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> Self where R: Request<S, A> {
        Registration::new(uri, Method::Head, handler)
    }

    pub fn uri(&self) -> &impl AsRef<str> {
        &self.uri
    }

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn handler<A>(self) -> Box<dyn for <'r> Fn(&'r mut R) -> anyhow::Result<Response<S>>> where R: Request<S, A> {
        self.handler
    }
}

pub mod registry {
    use std::vec;

    use crate::httpd::{Request, Response, Registration, Method};

    pub trait Registry<R, S>: Sized {
        fn register(&mut self, registration: Registration<R, S>) -> anyhow::Result<()>;

        fn register_all(&mut self, registrations: vec::Vec<Registration<R, S>>) -> anyhow::Result<()> {
            for registration in registrations {
                self.register(registration)?
            }
    
            Ok(()) // TODO
        }
        
        fn at(self, uri: impl ToString) -> RegistrationBuilder<Self> {
            RegistrationBuilder {
                uri: uri.to_string(),
                registry: self,
            }
        }
    }
    
    pub struct RegistrationBuilder<RR> {
        uri: String,
        registry: RR,
    }
    
    impl<RR> RegistrationBuilder<RR> {
        pub fn get<R, S, A>(self, f: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> anyhow::Result<RR> 
            where 
                RR: Registry<R, S>, R: Request<S, A> {
            self.handler(Method::Get, f)
        }

        pub fn post<R, S, A>(self, f: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> anyhow::Result<RR> 
            where RR: Registry<R, S>, R: Request<S, A> {
            self.handler(Method::Post, f)
        }
        
        pub fn put<R, S, A>(self, f: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> anyhow::Result<RR> where RR: Registry<R, S>, R: Request<S, A> {
            self.handler(Method::Put, f)
        }
    
        pub fn delete<R, S, A>(self, f: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> anyhow::Result<RR> where RR: Registry<R, S>, R: Request<S, A> {
            self.handler(Method::Delete, f)
        }
    
        pub fn head<R, S, A>(self, f: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> anyhow::Result<RR> where RR: Registry<R, S>, R: Request<S, A> {
            self.handler(Method::Head, f)
        }
    
        pub fn handler<R, S, A>(mut self, method: Method, f: impl Fn(&mut R) -> anyhow::Result<Response<S>> + 'static) -> anyhow::Result<RR> where RR: Registry<R, S>, R: Request<S, A> {
            self.registry.register(Registration::new(self.uri, method, f))?;
    
            Ok(self.registry)
        }
    }
}

pub mod sessions_impl {
    use std::{collections::HashMap, fmt::Write};
    use crate::httpd::{Request, Response, ResponseBuilder};

    struct SessionData<D> {
        last_accessed: std::time::Instant,
        session_timeout: std::time::Duration,
        used: u32,
        data: D,
    }

    pub struct Sessions<D> {
        max_sessions: usize,
        data: HashMap<String, SessionData<D>>,
    }

    impl<D> Sessions<D> {
        pub fn new(max_sessions: usize) -> Self {
            Sessions {
                max_sessions,
                data: HashMap::new(),
            }
        }
        
        pub fn invalidate(&mut self, session_id: &str) -> bool {
            match self.data.remove(session_id) {
                Some(_) => true,
                None => false,
            }
        }

        pub fn get<S, A>(&mut self, req: &impl Request<S, A>) -> Option<D> where D: Clone {
            if let Some(session_id) = get_id(req) {
                if let Some(session_data) = self.data.get_mut(session_id.as_str()) {
                    let now = std::time::Instant::now();
        
                    if session_data.used > 0 || session_data.last_accessed + session_data.session_timeout > now {
                        session_data.last_accessed = now;
                        session_data.used += 1;
                        Some(session_data.data.clone())
                    } else {
                        self.data.remove(session_id.as_str());

                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }

        pub fn update<S, A>(&mut self, req: &impl Request<S, A>, mut resp: Response<S>, new: impl FnOnce(S) -> D) -> Response<S> {
            let session_id = get_id(req);

            if let Some(new_session) = resp.new_session {
                let new_sess = session_id.as_ref().map(String::as_str).map_or(true, |s| self.data.remove(s).is_none());
        
                if new_sess && self.data.len() == self.max_sessions {
                    ResponseBuilder::new(429).into()
                } else {
                    let new_session_id = generate_session_id();
                    
                    resp.headers.insert("set-cookie".into(), insert_session_cookie("", &new_session_id));
                    resp.new_session = None;
        
                    self.data.insert(new_session_id, SessionData {
                        last_accessed: std::time::Instant::now(),
                        session_timeout: std::time::Duration::from_secs(20 * 60),
                        used: 0,
                        data: new(new_session), 
                    });
        
                    resp
                }
            } else {
                if let Some(session_id) = session_id.as_ref().map(String::as_str) {
                    if let Some(session_data) = self.data.get_mut(session_id) {
                        session_data.last_accessed = std::time::Instant::now();
                        session_data.used -= 1;
                    }
                }

                resp
            }
        }

        pub fn cleanup(&mut self) {
            let now = std::time::Instant::now();

            self.data.retain(|_, sd| sd.last_accessed + sd.session_timeout > now);
        }
    }

    fn get_id<S, A>(req: &impl Request<S, A>) -> Option<String> {
        req
            .header("Cookie")
            .map(|v| parse_session_cookie(v.as_str()))
            .flatten()
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
