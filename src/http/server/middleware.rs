use super::{Handler, HandlerResult, Request, Response};

pub trait Middleware<R, S>: Send
where
    R: Request,
    S: Response,
{
    fn handle<H>(&self, req: R, resp: S, handler: &H) -> HandlerResult
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
//     M: Fn(R, S, H) -> HandlerResult + Clone + Send + Sync + 'static,
//     R: Request,
//     S: Response,
//     H: Handler<R, S> + Send + Sync + 'static,
// {
//     fn handle<H2>(&self, req: R, resp: S, handler: H2) -> HandlerResult {
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
    fn handle(&self, req: R, resp: S) -> HandlerResult {
        self.middleware.handle(req, resp, &self.handler)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::http::server::asynch::{Handler, HandlerResult, Request, Response};

    pub trait Middleware<R, S>: Send
    where
        R: Request,
        S: Response,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult> + Send
        where
            Self: 'a;

        fn handle<H>(&self, req: R, resp: S, handler: &H) -> Self::HandleFuture<'_>
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
    //     M: Fn(R, S, H) -> HandlerResult + Clone + Send + Sync + 'static,
    //     R: Request,
    //     S: Response,
    //     H: Handler<R, S> + Send + Sync + 'static,
    // {
    //     fn handle<H2>(&self, req: R, resp: S, handler: H2) -> HandlerResult {
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
        type HandleFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = HandlerResult> + Send + 'a;

        fn handle(&self, req: R, resp: S) -> Self::HandleFuture<'_> {
            self.middleware.handle(req, resp, &self.handler)
        }
    }
}
