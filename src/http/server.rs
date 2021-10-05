extern crate alloc;
use alloc::borrow::Cow;
use alloc::string::String;

use crate::io::{self, Write};

use super::{HttpHeaders, HttpMethod, HttpSendHeaders, HttpSendStatus};

pub trait HttpRequest<'a>: HttpHeaders {
    type Read: io::Read<Error = Self::Error>;

    #[cfg(not(feature = "std"))]
    type Error;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn query_string(&self) -> Cow<'a, str>;

    fn payload(&mut self) -> &mut Self::Read;
}

pub trait HttpResponse<'a>: HttpSendStatus<'a> + HttpSendHeaders<'a> {
    type Write: io::Write<Error = Self::Error>;

    #[cfg(not(feature = "std"))]
    type Error;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn send_bytes(
        self,
        request: impl HttpRequest<'a>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<HttpCompletion, Self::Error>
    where
        Self: Sized,
    {
        self.send(request, |write| write.do_write_all(bytes.as_ref()))
    }

    fn send_str(
        self,
        request: impl HttpRequest<'a>,
        s: impl AsRef<str>,
    ) -> Result<HttpCompletion, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(request, s.as_ref().as_bytes())
    }

    fn send_json<T>(
        self,
        _request: impl HttpRequest<'a>,
        _t: impl AsRef<T>,
    ) -> Result<HttpCompletion, Self::Error>
    where
        Self: Sized,
    {
        todo!()
    }

    fn send_reader<R: io::Read<Error = Self::Error>>(
        self,
        request: impl HttpRequest<'a>,
        size: usize,
        read: R,
    ) -> Result<HttpCompletion, Self::Error>
    where
        Self: Sized,
    {
        self.send(request, |write| {
            io::copy_len(read, write, size as u64)?;

            Ok(())
        })
    }

    fn send(
        self,
        request: impl HttpRequest<'a>,
        f: impl FnOnce(&mut Self::Write) -> Result<(), Self::Error>,
    ) -> Result<HttpCompletion, Self::Error>
    where
        Self: Sized;

    fn submit(self, request: impl HttpRequest<'a>) -> Result<HttpCompletion, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(request, &[0_u8; 0])
    }
}

struct PrivateData;

pub struct HttpCompletion(PrivateData);

impl HttpCompletion {
    pub fn new<'a>(_req: impl HttpRequest<'a>, _resp: impl HttpResponse<'a>) -> Self {
        Self(PrivateData)
    }
}

pub struct HttpHandler<H> {
    uri: String,
    method: HttpMethod,
    handler: H,
}

impl<H> HttpHandler<H> {
    pub fn new(uri: impl Into<String>, method: HttpMethod, handler: H) -> Self {
        Self {
            uri: uri.into(),
            method,
            handler,
        }
    }

    pub fn uri(&self) -> &impl AsRef<str> {
        &self.uri
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn handler(self) -> H {
        self.handler
    }
}

pub trait HttpRegistry: Sized {
    type Request<'a>: HttpRequest<'a>;
    type Response<'a>: HttpResponse<'a>;

    #[cfg(not(feature = "std"))]
    type Error;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn set_handler<'a, F, E>(&mut self, handler: HttpHandler<F>) -> Result<&mut Self, Self::Error>
    where
        F: Fn(Self::Request<'a>, Self::Response<'a>) -> Result<HttpCompletion, E>,
        E: Into<Box<dyn std::error::Error>>;

    fn at(self, uri: impl ToString) -> HttpRegistryBuilder<Self> {
        HttpRegistryBuilder {
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

pub struct HttpRegistryBuilder<RR> {
    uri: String,
    registry: RR,
}

impl<RR> HttpRegistryBuilder<RR>
where
    RR: HttpRegistry,
{
    pub fn get<'a, E>(
        self,
        f: impl Fn(RR::Request<'a>, RR::Response<'a>) -> Result<HttpCompletion, E>,
    ) -> Result<RR, RR::Error>
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        self.handler(HttpMethod::Get, f)
    }

    pub fn post<'a, E>(
        self,
        f: impl Fn(RR::Request<'a>, RR::Response<'a>) -> Result<HttpCompletion, E>,
    ) -> Result<RR, RR::Error>
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        self.handler(HttpMethod::Post, f)
    }

    pub fn handler<'a, E>(
        mut self,
        method: HttpMethod,
        f: impl Fn(RR::Request<'a>, RR::Response<'a>) -> Result<HttpCompletion, E>,
    ) -> Result<RR, RR::Error>
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        self.registry
            .set_handler(HttpHandler::new(self.uri, method, f))?;

        Ok(self.registry)
    }
}

// fn test<'a>(
//     req: impl HttpRequest<'a>,
//     resp: impl HttpResponse<'a>,
// ) -> Result<HttpCompletion, anyhow::Error> {
//     Ok(resp.send_str(req, "Hello, world!")?)
// }

// fn blah<R>(registry: R) -> Result<R, R::Error>
// where
//     R: HttpRegistry,
// {
//     registry.at("/blah").get(test)
// }
