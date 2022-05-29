use core::fmt::Debug;

use super::{registry::*, *};

pub trait Middleware<C>: Send
where
    C: Context,
{
    type Error: Debug;

    fn handle<H, E>(
        &self,
        req: C::Request,
        resp: C::Response,
        handler: H,
    ) -> Result<Completion, EitherError<Self::Error, E>>
    where
        H: Fn(C::Request, C::Response) -> Result<Completion, E> + Send + Sync,
        E: Debug;

    fn compose<M>(self, middleware: M) -> CompositeMiddleware<Self, M>
    where
        M: Middleware<C> + Clone,
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

impl<C, M1, M2> Middleware<C> for CompositeMiddleware<M1, M2>
where
    C: Context,
    M1: Middleware<C>,
    M2: Middleware<C> + Clone + Send + Sync,
{
    type Error = EitherError<M1::Error, M2::Error>;

    fn handle<H, E>(
        &self,
        req: C::Request,
        resp: C::Response,
        handler: H,
    ) -> Result<Completion, EitherError<Self::Error, E>>
    where
        H: Fn(C::Request, C::Response) -> Result<Completion, E> + Send + Sync,
        E: Debug,
    {
        let middleware2 = self.middleware2.clone();

        self.middleware1
            .handle(req, resp, move |req, resp| {
                middleware2.handle(req, resp, &handler)
            })
            .map_err(|e| match e {
                EitherError::First(e) => EitherError::First(EitherError::First(e)),
                EitherError::Second(EitherError::First(e)) => {
                    EitherError::First(EitherError::Second(e))
                }
                EitherError::Second(EitherError::Second(e)) => EitherError::Second(e),
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
    M: Middleware<R> + Clone,
{
    pub fn new(registry: &'r mut R, middleware: M) -> Self {
        Self {
            registry,
            middleware,
        }
    }
}

impl<'r, R, M> Errors for MiddlewareRegistry<'r, R, M>
where
    R: Registry,
    M: Middleware<R> + Clone,
{
    type Error = R::Error;
}

impl<'r, R, M> Context for MiddlewareRegistry<'r, R, M>
where
    R: Registry,
    M: Middleware<R> + Clone + Send + Sync + 'static,
{
    type Request = R::Request;

    type Response = R::Response;
}

impl<'r, R, M> Registry for MiddlewareRegistry<'r, R, M>
where
    R: Registry,
    M: Middleware<R> + Clone + Send + Sync + 'static,
{
    type Root = R;

    type MiddlewareRegistry<'q, M2>
    where
        Self: 'q,
        M2: Middleware<Self::Root> + Clone + 'q + Send + Sync + 'static,
    = MiddlewareRegistry<'q, Self::Root, CompositeMiddleware<M, M2>>;

    fn with_middleware<M2>(&mut self, middleware: M2) -> Self::MiddlewareRegistry<'_, M2>
    where
        M2: middleware::Middleware<Self::Root> + Clone + Send + Sync + 'static,
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
        H: Fn(Self::Request, Self::Response) -> Result<Completion, E> + Send + Sync + 'static,
        E: Debug,
    {
        let middleware = self.middleware.clone();

        self.registry
            .set_inline_handler(uri, method, move |req, resp| {
                middleware.handle(req, resp, |req, resp| handler(req, resp))
            })?;

        Ok(self)
    }
}
