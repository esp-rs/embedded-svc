extern crate alloc;
use alloc::borrow::Cow;
use alloc::string::String;

use crate::io::{self, Write};

use super::{Headers, Method, SendHeaders, SendStatus};

pub trait Request<'a>: Headers {
    type Read: io::Read<Error = Self::Error>;
    type Error: std::error::Error + Send + Sync + 'static;

    fn query_string(&self) -> Cow<'a, str>;

    fn payload(&mut self) -> &mut Self::Read;
}

pub trait Response<'a>: SendStatus<'a> + SendHeaders<'a> {
    type Write: io::Write<Error = Self::Error>;
    type Error: std::error::Error + Send + Sync + 'static;

    fn send_bytes(
        self,
        request: impl Request<'a>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<Completion, Self::Error> {
        self.send(request, |w| w.do_write_all(bytes.as_ref()))
    }

    fn send_str(
        self,
        request: impl Request<'a>,
        s: impl AsRef<str>,
    ) -> Result<Completion, Self::Error> {
        self.send_bytes(request, s.as_ref().as_bytes())
    }

    fn send_json<T>(
        self,
        _request: impl Request<'a>,
        _t: impl AsRef<T>,
    ) -> Result<Completion, Self::Error> {
        todo!()
    }

    fn send_reader<R: io::Read<Error = Self::Error>>(
        self,
        request: impl Request<'a>,
        size: usize,
        mut read: impl AsMut<R>,
    ) -> Result<Completion, Self::Error> {
        self.send(request, |write| {
            read.as_mut().do_copy_len(size as u64, write)?;

            Ok(())
        })
    }

    fn send(
        self,
        request: impl Request<'a>,
        f: impl FnOnce(&mut Self::Write) -> Result<(), Self::Error>,
    ) -> Result<Completion, Self::Error>;

    fn submit(self, request: impl Request<'a>) -> Result<Completion, Self::Error> {
        self.send_bytes(request, &[0_u8; 0])
    }
}

struct PrivateData;

pub struct Completion(PrivateData);

impl Completion {
    pub fn new<'a>(_req: impl Request<'a>, _resp: impl Response<'a>) -> Self {
        Self(PrivateData)
    }
}

pub struct Handler<H> {
    uri: String,
    method: Method,
    handler: H,
}

impl<H> Handler<H> {
    pub fn new(uri: impl ToString, method: Method, handler: H) -> Self {
        Handler {
            uri: uri.to_string(),
            method,
            handler,
        }
    }

    pub fn uri(&self) -> &impl AsRef<str> {
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
    type Response<'a>: Response<'a>;
    type Error: std::error::Error + Send + Sync + 'static;

    fn set_handler<'a, F, E>(&mut self, handler: Handler<F>) -> Result<&mut Self, Self::Error>
    where
        F: Fn(Self::Request<'a>, Self::Response<'a>) -> Result<Completion, E>,
        E: Into<Box<dyn std::error::Error>>;

    fn at(self, uri: impl ToString) -> RegistryBuilder<Self> {
        RegistryBuilder {
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

pub struct RegistryBuilder<RR> {
    uri: String,
    registry: RR,
}

impl<RR> RegistryBuilder<RR>
where
    RR: Registry,
{
    pub fn get<'a, E>(
        self,
        f: impl Fn(RR::Request<'a>, RR::Response<'a>) -> Result<Completion, E>,
    ) -> Result<RR, RR::Error>
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        self.handler(Method::Get, f)
    }

    pub fn post<'a, E>(
        self,
        f: impl Fn(RR::Request<'a>, RR::Response<'a>) -> Result<Completion, E>,
    ) -> Result<RR, RR::Error>
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        self.handler(Method::Post, f)
    }

    pub fn handler<'a, E>(
        mut self,
        method: Method,
        f: impl Fn(RR::Request<'a>, RR::Response<'a>) -> Result<Completion, E>,
    ) -> Result<RR, RR::Error>
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        self.registry
            .set_handler(Handler::new(self.uri, method, f))?;

        Ok(self.registry)
    }
}

// fn test<'a>(req: impl Request<'a>, resp: impl Response<'a>) -> Result<Completion, anyhow::Error> {
//     Ok(resp.send_str(req, "Hello, world!")?)
// }

// fn blah<R>(registry: R) -> Result<R, R::Error>
// where
//     R: Registry,
// {
//     registry.at("/blah").get(test)
// }
