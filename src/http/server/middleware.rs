extern crate alloc;
use alloc::string::String;
use alloc::sync::Arc;

use super::{
    registry::{handle, Registry},
    *,
};

pub trait Middleware<R>
where
    R: Registry,
{
    type Error: fmt::Display + fmt::Debug;

    fn handle<'a, H, E>(
        &self,
        req: R::Request<'a>,
        resp: R::Response<'a>,
        handler: H,
    ) -> Result<Completion, Self::Error>
    where
        R: Registry,
        H: FnOnce(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Display + fmt::Debug;
}

pub struct MiddlewareRegistry<'r, R, M> {
    registry: &'r mut R,
    middleware: M,
}

impl<'r, R, M> MiddlewareRegistry<'r, R, M>
where
    R: Registry,
    M: Middleware<R> + Clone + 'static,
    M::Error: 'static,
{
    pub(crate) fn new(registry: &'r mut R, middleware: M) -> Self {
        Self {
            registry,
            middleware,
        }
    }

    // TODO
    // fn with_middleware(&mut self, middleware: M) -> middleware::MiddlewareRegistry<'r, R, M> {
    //     middleware::MiddlewareRegistry::new(self.registry, Self::combine(self.middleware.clone(), middleware))
    // }

    pub fn at(&mut self, uri: impl ToString) -> MiddlewareHandlerRegistrationBuilder<'_, 'r, R, M> {
        MiddlewareHandlerRegistrationBuilder {
            uri: uri.to_string(),
            middleware_registry: self,
        }
    }

    fn set_inline_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        let middleware = self.middleware.clone();

        // TODO: Terrible...
        let handler = Arc::new(handler);

        self.registry
            .set_inline_handler(uri, method, move |req, resp| {
                let mhandler = handler.clone();

                middleware.handle(req, resp, move |req, resp| mhandler(req, resp))
            })?;

        Ok(self)
    }

    fn set_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, R::Error>
    where
        H: for<'a> Fn(&mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.set_inline_handler(uri, method, move |req, resp| {
            handle::<R, _, _>(req, resp, &handler)
        })
    }

    // TODO
    // fn combine(middleware1: M, middleware2: M) -> M {
    //     todo!()
    // }
}

pub struct MiddlewareInlineHandlerRegistrationBuilder<'m, 'r, R, M> {
    uri: String,
    middleware_registry: &'m mut MiddlewareRegistry<'r, R, M>,
}

impl<'m, 'r, R, M> MiddlewareInlineHandlerRegistrationBuilder<'m, 'r, R, M>
where
    R: Registry,
    M: Middleware<R> + Clone + 'static,
    M::Error: 'static,
{
    pub fn get<H, E>(self, handler: H) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Get, handler)
    }

    pub fn put<H, E>(self, handler: H) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Put, handler)
    }

    pub fn post<H, E>(self, handler: H) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Post, handler)
    }

    pub fn delete<H, E>(self, handler: H) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Delete, handler)
    }

    pub fn handler<H, E>(
        self,
        method: Method,
        handler: H,
    ) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.middleware_registry
            .set_inline_handler(self.uri.as_str(), method, handler)
    }
}

pub struct MiddlewareHandlerRegistrationBuilder<'m, 'r, R, M> {
    uri: String,
    middleware_registry: &'m mut MiddlewareRegistry<'r, R, M>,
}

impl<'m, 'r, R, M> MiddlewareHandlerRegistrationBuilder<'m, 'r, R, M>
where
    R: Registry,
    M: Middleware<R> + Clone + 'static,
    M::Error: 'static,
{
    pub fn inline(self) -> MiddlewareInlineHandlerRegistrationBuilder<'m, 'r, R, M> {
        MiddlewareInlineHandlerRegistrationBuilder {
            uri: self.uri,
            middleware_registry: self.middleware_registry,
        }
    }

    pub fn get<H, E>(self, handler: H) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Get, handler)
    }

    pub fn put<H, E>(self, handler: H) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Put, handler)
    }

    pub fn post<H, E>(self, handler: H) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Put, handler)
    }

    pub fn delete<H, E>(self, handler: H) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Put, handler)
    }

    pub fn handler<H, E>(
        self,
        method: Method,
        handler: H,
    ) -> Result<&'m mut MiddlewareRegistry<'r, R, M>, R::Error>
    where
        H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.middleware_registry
            .set_handler(self.uri.as_str(), method, handler)
    }
}
