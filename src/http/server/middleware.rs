extern crate alloc;
use alloc::sync::Arc;

use super::{registry::*, *};

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

    fn compose<M>(self, middleware: M) -> CompositeMiddleware<Self, M>
    where
        M: Middleware<R> + Clone + 'static,
        Self: Sized,
    {
        CompositeMiddleware::new(self, middleware)
    }
}

pub struct CompositeMiddleware<M1, M2> {
    middleware1: M1,
    middleware2: M2,
}

impl<M1, M2> CompositeMiddleware<M1, M2> {
    pub fn new(middleware1: M1, middleware2: M2) -> Self {
        Self {
            middleware1,
            middleware2,
        }
    }
}

impl<M1, M2> Clone for CompositeMiddleware<M1, M2>
where
    M1: Clone,
    M2: Clone,
{
    fn clone(&self) -> Self {
        Self {
            middleware1: self.middleware1.clone(),
            middleware2: self.middleware2.clone(),
        }
    }
}

impl<R, M1, M2> Middleware<R> for CompositeMiddleware<M1, M2>
where
    R: Registry,
    M1: Middleware<R>,
    M2: Middleware<R> + Clone + 'static,
{
    type Error = M1::Error;

    fn handle<'a, H, E>(
        &self,
        req: R::Request<'a>,
        resp: R::Response<'a>,
        handler: H,
    ) -> Result<Completion, Self::Error>
    where
        H: FnOnce(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Display + fmt::Debug,
    {
        let middleware2 = self.middleware2.clone();

        self.middleware1.handle(req, resp, move |req, resp| {
            middleware2.handle(req, resp, handler)
        })
    }
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
    M: Middleware<R> + Clone + 'static,
    M::Error: 'static,
{
    type Request<'a> = R::Request<'a>;

    type Response<'a> = R::Response<'a>;

    type Error = R::Error;

    type Root = R;

    type MiddlewareRegistry<'q, M2>
    where
        Self: 'q,
        M2: Middleware<Self::Root> + Clone + 'static + 'q,
    = MiddlewareRegistry<'q, Self::Root, CompositeMiddleware<M, M2>>;

    fn with_middleware<M2>(&mut self, middleware: M2) -> Self::MiddlewareRegistry<'_, M2>
    where
        M2: middleware::Middleware<Self::Root> + Clone + 'static,
        M2::Error: 'static,
        Self: Sized,
    {
        middleware::MiddlewareRegistry::new(
            self.registry,
            self.middleware.clone().compose(middleware),
        )
    }

    #[allow(clippy::redundant_closure)]
    fn set_inline_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::Response<'a>) -> Result<Completion, E> + 'static,
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
}
