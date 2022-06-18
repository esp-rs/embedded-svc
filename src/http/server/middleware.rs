use super::{Handler, HandlerError, Request, Response};

pub trait Middleware<R, S>: Send
where
    R: Request,
    S: Response,
{
    fn handle<H>(&self, req: R, resp: S, handler: &H) -> Result<(), HandlerError>
    where
        H: Handler<R, S>;

    fn compose<H>(self, handler: H) -> CompositeHandler<Self, H>
    where
        H: Handler<R, S>,
        Self: Sized,
    {
        CompositeHandler::new(self, handler)
    }
}

// impl<M, R, S, H> Middleware<R, S> for M
// where
//     M: Fn(R, S, H) -> Result<(), HandlerError> + Clone + Send + Sync + 'static,
//     R: Request,
//     S: Response,
//     H: Handler<R, S> + Send + Sync + 'static,
// {
//     fn handle<H2>(&self, req: R, resp: S, handler: H2) -> Result<(), HandlerError> {
//         (self)(req, resp, handler)
//     }
// }

pub struct CompositeHandler<M, H> {
    middleware: M,
    handler: H,
}

impl<M, H> CompositeHandler<M, H> {
    pub fn new(middleware: M, handler: H) -> Self {
        Self {
            middleware,
            handler,
        }
    }
}

impl<M, H, R, S> Handler<R, S> for CompositeHandler<M, H>
where
    M: Middleware<R, S>,
    H: Handler<R, S>,
    R: Request,
    S: Response,
{
    fn handle(&self, req: R, resp: S) -> Result<(), HandlerError> {
        self.middleware.handle(req, resp, &self.handler)
    }
}
