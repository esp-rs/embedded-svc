extern crate alloc;
use alloc::string::{String, ToString};

use super::{middleware, *};

pub trait Registry {
    type Request<'a>: Request<'a>;

    type Response<'a>: Response<'a>;

    #[cfg(not(feature = "std"))]
    type Error: fmt::Debug + fmt::Display;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    type Root: for<'a> Registry<
        Request = Self::Request<'a>,
        Response = Self::Response<'a>,
        Error = Self::Error,
    >;

    type MiddlewareRegistry<'q, M>: for<'a> Registry<
        Request = Self::Request<'a>,
        Response = Self::Response<'a>,
        Error = Self::Error,
    >
    where
        Self: 'q,
        M: middleware::Middleware<Self::Root> + Clone + 'static + 'q;

    fn with_middleware<M>(&mut self, middleware: M) -> Self::MiddlewareRegistry<'_, M>
    // middleware::MiddlewareRegistry<'_, Self, M>
    where
        M: middleware::Middleware<Self::Root> + Clone + 'static,
        M::Error: 'static,
        Self: Sized;
    // {
    //     middleware::MiddlewareRegistry::new(self, middleware)
    // }

    fn at(&mut self, uri: impl ToString) -> HandlerRegistrationBuilder<Self>
    where
        Self: Sized,
    {
        HandlerRegistrationBuilder {
            uri: uri.to_string(),
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
        H: for<'a> Fn(Self::Request<'a>, Self::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Display + fmt::Debug;

    fn set_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        Self: Sized,
        H: for<'a, 'c> Fn(&'c mut Self::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<Self as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.set_inline_handler(uri, method, move |req, resp| {
            handle::<Self, _, _>(req, resp, &handler)
        })
    }
}

pub struct InlineHandlerRegistrationBuilder<'r, R> {
    uri: String,
    registry: &'r mut R,
}

impl<'r, R> InlineHandlerRegistrationBuilder<'r, R>
where
    R: Registry,
{
    pub fn get<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Get, handler)
    }

    pub fn put<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Put, handler)
    }

    pub fn post<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Post, handler)
    }

    pub fn delete<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Delete, handler)
    }

    pub fn handler<H, E>(self, method: Method, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a> Fn(R::Request<'a>, R::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.registry
            .set_inline_handler(self.uri.as_str(), method, handler)
    }
}

pub struct HandlerRegistrationBuilder<'r, R> {
    uri: String,
    registry: &'r mut R,
}

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
        H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            // TODO: Remove the constraints below as they cannot be easily handled in no_std
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Get, handler)
    }

    pub fn put<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
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

    pub fn post<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Post, handler)
    }

    pub fn delete<H, E>(self, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Delete, handler)
    }

    pub fn handler<H, E>(self, method: Method, handler: H) -> Result<&'r mut R, R::Error>
    where
        H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<R as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.registry
            .set_handler(self.uri.as_str(), method, handler)?;

        Ok(self.registry)
    }
}

fn handle<'b, R, H, E>(
    mut req: R::Request<'b>,
    mut inline_resp: R::Response<'b>,
    handler: &H,
) -> Result<Completion, E>
where
    R: Registry,
    H: for<'a, 'c> Fn(&'c mut R::Request<'a>) -> Result<ResponseData, E>,
    E: fmt::Debug
        + fmt::Display
        + From<<<R as Registry>::Response<'b> as Response<'b>>::Error>
        + From<io::IODynError>,
{
    let resp = handler(&mut req)?;

    inline_resp.set_status(resp.status);

    if let Some(status_message) = resp.status_message {
        inline_resp.set_status_message(status_message);
    }

    for (key, value) in resp.headers {
        inline_resp.set_header(key, value);
    }

    match resp.body {
        Body::Empty => inline_resp.submit(req).map_err(Into::into),
        Body::Bytes(bytes) => inline_resp.send_bytes(req, &bytes).map_err(Into::into),
        Body::Read(size, reader) => {
            inline_resp
                .send_reader(req, size, reader)
                .map_err(|e| match e {
                    SendError::SendError(e) => e.into(),
                    SendError::WriteError(e) => e.into(),
                })
        }
    }
}
