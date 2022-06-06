use core::fmt::Debug;

use super::{middleware, *};
use crate::errors::wrap::EitherError3;
use crate::io::ErrorKind;

pub trait PrefixedRegistry: Registry {
    fn prefix<'a>(&'a mut self, prefix: &'a str) -> Self
    where
        Self: Sized;
}

pub trait Registry: Context {
    type Context: Context<Request = Self::Request, Response = Self::Response, Error = Self::Error>;

    type MiddlewareRegistry<'q, M>: Registry<
        Request = Self::Request,
        Response = Self::Response,
        Error = Self::Error,
    >
    where
        Self: 'q,
        M: middleware::Middleware<Self::Context> + Clone + Send + Sync + 'static + 'q;

    fn with_middleware<M>(&mut self, middleware: M) -> Self::MiddlewareRegistry<'_, M>
    where
        M: middleware::Middleware<Self::Context> + Clone + Send + Sync + 'static,
        Self: Sized;

    fn at<'a>(&'a mut self, uri: &'a str) -> HandlerRegistrationBuilder<'a, Self>
    where
        Self: Sized,
    {
        HandlerRegistrationBuilder {
            uri,
            registry: self,
        }
    }

    fn set_inline_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        H: Fn(Self::Request, Self::Response) -> Result<Completion, E> + Send + Sync + 'static,
        E: Debug;

    #[cfg(feature = "alloc")]
    fn set_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        Self: Sized,
        H: for<'a> Fn(&'a mut Self::Request) -> Result<ResponseData, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.set_inline_handler(uri, method, move |req, resp| {
            handle::<Self, _, _>(req, resp, &handler)
        })
    }
}

pub struct InlineHandlerRegistrationBuilder<'r, R> {
    uri: &'r str,
    registry: &'r mut R,
}

impl<'r, R> InlineHandlerRegistrationBuilder<'r, R>
where
    R: Registry,
{
    pub fn get<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: Fn(R::Request, R::Response) -> Result<Completion, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.handler(Method::Get, handler)
    }

    pub fn put<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: Fn(R::Request, R::Response) -> Result<Completion, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.handler(Method::Put, handler)
    }

    pub fn post<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: Fn(R::Request, R::Response) -> Result<Completion, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.handler(Method::Post, handler)
    }

    pub fn delete<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: Fn(R::Request, R::Response) -> Result<Completion, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.handler(Method::Delete, handler)
    }

    pub fn handler<H, E>(self, method: Method, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: Fn(R::Request, R::Response) -> Result<Completion, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.registry.set_inline_handler(self.uri, method, handler)
    }
}

#[cfg(feature = "alloc")]
pub struct HandlerRegistrationBuilder<'r, R> {
    uri: &'r str,
    registry: &'r mut R,
}

#[cfg(feature = "alloc")]
impl<'r, R> HandlerRegistrationBuilder<'r, R>
where
    R: Registry,
{
    pub fn inline(self) -> InlineHandlerRegistrationBuilder<'r, R> {
        InlineHandlerRegistrationBuilder {
            uri: self.uri,
            registry: self.registry,
        }
    }

    pub fn get<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(&'a mut R::Request) -> Result<ResponseData, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.handler(Method::Get, handler)
    }

    pub fn put<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(&'a mut R::Request) -> Result<ResponseData, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.handler(Method::Put, handler)
    }

    pub fn post<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(&'a mut R::Request) -> Result<ResponseData, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.handler(Method::Post, handler)
    }

    pub fn delete<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(&'a mut R::Request) -> Result<ResponseData, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.handler(Method::Delete, handler)
    }

    pub fn handler<H, E>(self, method: Method, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(&'a mut R::Request) -> Result<ResponseData, E> + Send + Sync + 'static,
        E: Debug,
    {
        self.registry.set_handler(self.uri, method, handler)?;

        Ok(self.registry)
    }
}

#[cfg(feature = "alloc")]
fn handle<R, H, E>(
    mut req: R::Request,
    mut inline_resp: R::Response,
    handler: &H,
) -> Result<Completion, EitherError3<E, R::Error, ErrorKind>>
where
    R: Registry,
    H: for<'a> Fn(&'a mut R::Request) -> Result<ResponseData, E> + Send + Sync,
    E: Debug,
{
    let resp = handler(&mut req).map_err(EitherError3::E1)?;

    inline_resp.set_status(resp.status);

    if let Some(status_message) = resp.status_message {
        inline_resp.set_status_message(&status_message);
    }

    for (key, value) in resp.headers {
        inline_resp.set_header(&key, &value);
    }

    match resp.body {
        Body::Empty => inline_resp.submit(req).map_err(EitherError3::E2),
        Body::Bytes(bytes) => inline_resp
            .send_bytes(req, &bytes)
            .map_err(EitherError3::E2),
        Body::Read(size, reader) => {
            inline_resp
                .send_reader(req, size, reader)
                .map_err(|e| match e {
                    EitherError::E1(e) => EitherError3::E2(e),
                    EitherError::E2(e) => EitherError3::E3(e),
                })
        }
    }
}
