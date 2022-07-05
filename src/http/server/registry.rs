use core::fmt::Debug;
use core::fmt::Write;

use crate::http::Method;
use crate::io::Error;

use super::FnHandler;
use super::{middleware::Middleware, Handler, HandlerResult, Request, Response};

pub trait Registry {
    type Error: Debug;
    type IOError: Error;

    type Request<'a>: Request<Error = Self::IOError>;
    type Response<'a>: Response<Error = Self::IOError>;

    fn handle_get<H>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::Response<'a>) -> HandlerResult + Send + 'static,
    {
        self.handle(uri, Method::Get, handler)
    }

    fn handle_post<H>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::Response<'a>) -> HandlerResult + Send + 'static,
    {
        self.handle(uri, Method::Post, handler)
    }

    fn handle_put<H>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::Response<'a>) -> HandlerResult + Send + 'static,
    {
        self.handle(uri, Method::Put, handler)
    }

    fn handle_delete<H>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::Response<'a>) -> HandlerResult + Send + 'static,
    {
        self.handle(uri, Method::Delete, handler)
    }

    fn handle<H>(&mut self, uri: &str, method: Method, handler: H) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::Response<'a>) -> HandlerResult + Send + 'static,
    {
        self.set_handler(uri, method, FnHandler::new(handler))
    }

    fn set_handler<H>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static;
}

pub struct PrefixedRegistry<'r, R, const N: usize = 128> {
    registry: &'r mut R,
    prefix: &'r str,
}

impl<'r, R, const N: usize> PrefixedRegistry<'r, R, N> {
    pub fn new(registry: &'r mut R, prefix: &'r str) -> Self {
        Self { registry, prefix }
    }
}

impl<'r, R, const N: usize> Registry for PrefixedRegistry<'r, R, N>
where
    R: Registry,
{
    type Error = R::Error;

    type IOError = R::IOError;

    type Request<'a> = R::Request<'a>;

    type Response<'a> = R::Response<'a>;

    fn set_handler<H>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static,
    {
        let mut prefixed_uri = heapless::String::<N>::new();

        write!(&mut prefixed_uri, "{}{}", self.prefix, uri).unwrap();

        self.registry.set_handler(&prefixed_uri, method, handler)?;

        Ok(self)
    }
}

pub struct MiddlewareRegistry<'r, R, M> {
    registry: &'r mut R,
    middleware: M,
}

impl<'r, R, M> MiddlewareRegistry<'r, R, M> {
    pub fn new(registry: &'r mut R, middleware: M) -> Self {
        Self {
            registry,
            middleware,
        }
    }
}

impl<'r, R, M> Registry for MiddlewareRegistry<'r, R, M>
where
    R: Registry,
    M: for<'a> Middleware<R::Request<'a>, R::Response<'a>> + Clone + 'static,
{
    type Error = R::Error;

    type IOError = R::IOError;

    type Request<'a> = R::Request<'a>;

    type Response<'a> = R::Response<'a>;

    fn set_handler<H>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static,
    {
        self.registry
            .set_handler(uri, method, self.middleware.clone().compose(handler))?;

        Ok(self)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::fmt::Debug;

    use crate::http::server::asynch::{Handler, Request, Response};
    use crate::http::*;
    use crate::io::Error;

    use crate::http::server::middleware::asynch::*;

    pub trait Registry {
        type Error: Debug;
        type IOError: Error;

        type Request<'a>: Request<Error = Self::IOError>;
        type Response<'a>: Response<Error = Self::IOError>;

        fn handle_get<H>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
        where
            H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static,
        {
            self.handle(uri, Method::Get, handler)
        }

        fn handle_post<H>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
        where
            H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static,
        {
            self.handle(uri, Method::Post, handler)
        }

        fn handle_put<H>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
        where
            H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static,
        {
            self.handle(uri, Method::Put, handler)
        }

        fn handle_delete<H>(&mut self, uri: &str, handler: H) -> Result<&mut Self, Self::Error>
        where
            H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static,
        {
            self.handle(uri, Method::Delete, handler)
        }

        fn handle<H>(
            &mut self,
            uri: &str,
            method: Method,
            handler: H,
        ) -> Result<&mut Self, Self::Error>
        where
            H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static,
        {
            self.set_handler(uri, method, handler)
        }

        fn set_handler<H>(
            &mut self,
            uri: &str,
            method: Method,
            handler: H,
        ) -> Result<&mut Self, Self::Error>
        where
            H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static;
    }

    pub struct PrefixedRegistry<'r, R, const N: usize = 128> {
        registry: &'r mut R,
        prefix: &'r str,
    }

    impl<'r, R, const N: usize> PrefixedRegistry<'r, R, N> {
        pub fn new(registry: &'r mut R, prefix: &'r str) -> Self {
            Self { registry, prefix }
        }
    }

    impl<'r, R, const N: usize> Registry for PrefixedRegistry<'r, R, N>
    where
        R: Registry,
    {
        type Error = R::Error;

        type IOError = R::IOError;

        type Request<'a> = R::Request<'a>;

        type Response<'a> = R::Response<'a>;

        fn set_handler<H>(
            &mut self,
            uri: &str,
            method: Method,
            handler: H,
        ) -> Result<&mut Self, Self::Error>
        where
            H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static,
        {
            let mut prefixed_uri = heapless::String::<N>::new();

            write!(&mut prefixed_uri, "{}{}", self.prefix, uri).unwrap();

            self.registry.set_handler(&prefixed_uri, method, handler)?;

            Ok(self)
        }
    }

    pub struct MiddlewareRegistry<'r, R, M> {
        registry: &'r mut R,
        middleware: M,
    }

    impl<'r, R, M> MiddlewareRegistry<'r, R, M> {
        pub fn new(registry: &'r mut R, middleware: M) -> Self {
            Self {
                registry,
                middleware,
            }
        }
    }

    impl<'r, R, M> Registry for MiddlewareRegistry<'r, R, M>
    where
        R: Registry,
        M: for<'a> Middleware<R::Request<'a>, R::Response<'a>> + Clone + 'static,
    {
        type Error = R::Error;

        type IOError = R::IOError;

        type Request<'a> = R::Request<'a>;

        type Response<'a> = R::Response<'a>;

        fn set_handler<H>(
            &mut self,
            uri: &str,
            method: Method,
            handler: H,
        ) -> Result<&mut Self, Self::Error>
        where
            H: for<'a> Handler<Self::Request<'a>, Self::Response<'a>> + 'static,
        {
            self.registry
                .set_handler(uri, method, self.middleware.clone().compose(handler))?;

            Ok(self)
        }
    }
}
